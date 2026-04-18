use std::{collections::HashMap, error::Error};

use crate::{
    Address, Locality, NumberRange, PublicSpace, encode_pc,
    parsing::{MunicipalityRelation, municipalities::Municipality},
};

pub struct LocalityMap {
    pub locality_names: Vec<String>,
    pub locality_codes: Vec<u16>,
    /// Parallel to `locality_names`: true when the source name carried a
    /// stripped province suffix (marker for non-unique reporting).
    pub locality_had_suffix: Vec<bool>,
    pub locality_map: HashMap<u16, u16>,
}

/// Build stable locality indexes keyed by BAG woonplaatsidentificatiecode.
///
/// Entries are deduplicated on the BAG id (not the name), so that two
/// Woonplaatsen sharing a name but differing in identificatiecode remain
/// distinct. Sorting is lexicographic on (name, id) for stable output.
pub fn index_localities(localities: Vec<Locality>) -> Result<LocalityMap, Box<dyn Error>> {
    let mut unique: HashMap<u16, (String, bool)> = HashMap::with_capacity(localities.len());
    for locality in localities {
        unique
            .entry(locality.id)
            .or_insert((locality.name, locality.had_suffix));
    }

    let mut entries: Vec<(String, u16, bool)> = unique
        .into_iter()
        .map(|(id, (name, had_suffix))| (name, id, had_suffix))
        .collect();
    entries.sort();

    if entries.len() > u16::MAX as usize {
        return Err("too many localities for u16 index".into());
    }

    let mut locality_names = Vec::with_capacity(entries.len());
    let mut locality_codes = Vec::with_capacity(entries.len());
    let mut locality_had_suffix = Vec::with_capacity(entries.len());
    let mut locality_map = HashMap::with_capacity(entries.len());
    for (index, (name, id, had_suffix)) in entries.into_iter().enumerate() {
        locality_map.insert(id, index as u16);
        locality_names.push(name);
        locality_codes.push(id);
        locality_had_suffix.push(had_suffix);
    }

    Ok(LocalityMap {
        locality_names,
        locality_codes,
        locality_had_suffix,
        locality_map,
    })
}

pub struct MunicipalityMap {
    pub municipality_names: Vec<String>,
    pub province_names: Vec<String>,
    pub municipality_codes: Vec<u16>,
    /// Parallel to `municipality_names`: true when the CBS name carried a
    /// stripped province suffix (marker for non-unique reporting).
    pub municipality_had_suffix: Vec<bool>,
    /// For each locality (by sorted locality_index), the municipality_index.
    pub locality_municipality: Vec<u16>,
    /// For each municipality (by sorted municipality_index), the province_index.
    pub municipality_province: Vec<u8>,
}

/// Build municipality and province indexes from GWR relations and CBS data.
pub fn index_municipalities(
    relations: Vec<MunicipalityRelation>,
    cbs_municipalities: &[Municipality],
    locality_map: &HashMap<u16, u16>,
    locality_count: usize,
) -> Result<MunicipalityMap, Box<dyn Error>> {
    // Build CBS lookup: gemeente_code -> (name, province, had_suffix)
    let mut cbs_lookup: HashMap<u16, (&str, &str, bool)> =
        HashMap::with_capacity(cbs_municipalities.len());
    for m in cbs_municipalities {
        cbs_lookup.insert(m.code, (&m.name, &m.province, m.had_suffix));
    }

    // Collect municipality codes reachable through locality relations.
    let mut reachable: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    for rel in &relations {
        if cbs_lookup.contains_key(&rel.municipality_code) {
            reachable.insert(rel.municipality_code);
        }
    }

    // Sort by (name, code) for stable output; dedup by code so municipalities
    // sharing a name (e.g. Bergen in NH and Limburg) remain distinct entries.
    let mut entries: Vec<(&str, u16, &str, bool)> = reachable
        .iter()
        .map(|&code| {
            let (name, prov, had_suffix) = cbs_lookup[&code];
            (name, code, prov, had_suffix)
        })
        .collect();
    entries.sort();

    if entries.len() > u16::MAX as usize {
        return Err("too many municipalities for u16 index".into());
    }

    let mut municipality_names = Vec::with_capacity(entries.len());
    let mut municipality_codes = Vec::with_capacity(entries.len());
    let mut municipality_had_suffix = Vec::with_capacity(entries.len());
    let mut code_to_index: HashMap<u16, u16> = HashMap::with_capacity(entries.len());
    for (i, (name, code, _, had_suffix)) in entries.iter().enumerate() {
        code_to_index.insert(*code, i as u16);
        municipality_names.push((*name).to_string());
        municipality_codes.push(*code);
        municipality_had_suffix.push(*had_suffix);
    }

    // Collect and deduplicate province codes (sorted)
    let mut province_names: Vec<String> =
        entries.iter().map(|(_, _, p, _)| p.to_string()).collect();
    province_names.sort();
    province_names.dedup();

    if province_names.len() > u8::MAX as usize {
        return Err("too many provinces for u8 index".into());
    }

    let mut province_name_index: HashMap<&str, u8> = HashMap::with_capacity(province_names.len());
    for (i, name) in province_names.iter().enumerate() {
        province_name_index.insert(name, i as u8);
    }

    // Build municipality_province: for each municipality_index, the province_index
    let mut municipality_province = vec![0u8; entries.len()];
    for (i, (_, _, prov, _)) in entries.iter().enumerate() {
        if let Some(&p_idx) = province_name_index.get(prov) {
            municipality_province[i] = p_idx;
        }
    }

    // Build locality_municipality: for each locality_index, the municipality_index
    // Use u16::MAX as sentinel for localities without a known municipality
    let mut locality_municipality = vec![u16::MAX; locality_count];
    for rel in &relations {
        if let Some(&locality_index) = locality_map.get(&rel.locality_id)
            && let Some(&m_idx) = code_to_index.get(&rel.municipality_code)
        {
            locality_municipality[locality_index as usize] = m_idx;
        }
    }

    Ok(MunicipalityMap {
        municipality_names,
        province_names,
        municipality_codes,
        municipality_had_suffix,
        locality_municipality,
        municipality_province,
    })
}

/// Build public space name indexes and map ids to locality indexes.
///
/// Public spaces referencing a locality that isn't in `locality_map` are
/// dropped — this happens when BAG keeps an issued street pointing to a
/// Woonplaats whose active lifecycle has ended (e.g. after a municipality
/// merger). We log the count so skew stays visible.
pub fn index_public_spaces(
    public_spaces: Vec<PublicSpace>,
    locality_map: HashMap<u16, u16>,
) -> (Vec<String>, HashMap<u64, (u32, u16)>) {
    let mut kept: Vec<PublicSpace> = Vec::with_capacity(public_spaces.len());
    let mut orphaned = 0usize;
    for public_space in public_spaces {
        if locality_map.contains_key(&public_space.locality_id) {
            kept.push(public_space);
        } else {
            orphaned += 1;
        }
    }
    if orphaned > 0 {
        eprintln!(
            "Warning: Dropped {orphaned} public space(s) referencing an unknown locality"
        );
    }

    let mut public_space_names: Vec<String> =
        kept.iter().map(|public_space| public_space.name.clone()).collect();
    public_space_names.sort();
    public_space_names.dedup();

    let mut name_index = HashMap::with_capacity(public_space_names.len());
    for (index, name) in public_space_names.iter().enumerate() {
        name_index.insert(name.clone(), index as u32);
    }

    let mut public_spaces_map = HashMap::with_capacity(kept.len());
    for public_space in kept {
        let public_space_index = *name_index
            .get(&public_space.name)
            .expect("Public space name not found in name index");
        let locality_index = *locality_map
            .get(&public_space.locality_id)
            .expect("Locality ID not found in locality map");

        public_spaces_map.insert(public_space.id, (public_space_index, locality_index));
    }

    (public_space_names, public_spaces_map)
}

/// Encode addresses into sorted, contiguous number ranges.
pub fn encode_addresses(
    addresses: Vec<Address>,
    public_spaces_map: &HashMap<u64, (u32, u16)>,
) -> Vec<NumberRange> {
    let mut entries = Vec::with_capacity(addresses.len());

    for address in addresses {
        let Some((public_space_index, locality_index)) =
            public_spaces_map.get(&address.public_space_id)
        else {
            continue;
        };

        let pc_encoded = encode_pc(address.postal_code.as_bytes());

        entries.push(EncodedEntry {
            postal_code: pc_encoded,
            house_number: address.house_number,
            public_space_index: *public_space_index,
            locality_index: *locality_index,
        });
    }

    entries.sort_by(|a, b| {
        a.postal_code
            .cmp(&b.postal_code)
            .then_with(|| a.public_space_index.cmp(&b.public_space_index))
            .then_with(|| a.locality_index.cmp(&b.locality_index))
            .then_with(|| a.house_number.cmp(&b.house_number))
    });

    let mut ranges = Vec::new();
    let mut current: Option<NumberRange> = None;

    for entry in entries {
        let EncodedEntry {
            postal_code,
            house_number,
            public_space_index,
            locality_index,
        } = entry;
        match current.as_mut() {
            Some(range)
                if range.postal_code == postal_code
                    && range.public_space_index == public_space_index
                    && range.locality_index == locality_index =>
            {
                let range_end = range.start + range.length as u32 * range.step as u32;
                if house_number <= range_end {
                    // Duplicate or already covered by the range
                    continue;
                }

                let diff = house_number - range_end;

                if range.length == 0 && diff <= u8::MAX as u32 {
                    // Second entry determines the step
                    let step = diff as u8;
                    range.step = step;
                    range.length = 1;
                } else if range.length < u16::MAX && diff == range.step as u32 {
                    // Continues the established step pattern
                    range.length += 1;
                } else {
                    let finished = current.take().expect("Range is missing");
                    ranges.push(finished);
                    current = Some(NumberRange {
                        postal_code,
                        start: house_number,
                        length: 0,
                        step: 1,
                        public_space_index,
                        locality_index,
                    });
                }
            }
            _ => {
                if let Some(finished) = current.take() {
                    ranges.push(finished);
                }
                current = Some(NumberRange {
                    postal_code,
                    start: house_number,
                    length: 0,
                    step: 1,
                    public_space_index,
                    locality_index,
                });
            }
        }
    }

    if let Some(finished) = current {
        ranges.push(finished);
    }

    ranges
}

struct EncodedEntry {
    postal_code: u32,
    house_number: u32,
    public_space_index: u32,
    locality_index: u16,
}

#[cfg(test)]
mod tests {
    use super::{LocalityMap, encode_addresses, index_localities, index_public_spaces};
    use crate::{Address, Locality, NumberRange, PublicSpace, encode_pc};

    fn locality_map_fixture() -> LocalityMap {
        let localities = vec![
            Locality {
                id: 10,
                name: "Beta".to_string(),
                had_suffix: false,
            },
            Locality {
                id: 11,
                name: "Alpha".to_string(),
                had_suffix: false,
            },
            Locality {
                id: 12,
                name: "Alpha".to_string(),
                had_suffix: false,
            },
        ];

        index_localities(localities).expect("locality map fixture")
    }

    #[test]
    fn index_localities_sorts_by_name_and_id() {
        let result = locality_map_fixture();

        // Two "Alpha" entries (ids 11, 12) remain distinct, sorted by id after name.
        assert_eq!(result.locality_names, vec!["Alpha", "Alpha", "Beta"]);
        assert_eq!(result.locality_codes, vec![11, 12, 10]);
        assert_eq!(result.locality_map.get(&11), Some(&0));
        assert_eq!(result.locality_map.get(&12), Some(&1));
        assert_eq!(result.locality_map.get(&10), Some(&2));
    }

    #[test]
    fn index_localities_rejects_overflow() {
        let localities = (0..=u16::MAX)
            .map(|id| Locality {
                id,
                name: format!("L{id}"),
                had_suffix: false,
            })
            .collect();

        let result = index_localities(localities);

        assert!(result.is_err());
    }

    #[test]
    fn index_public_spaces_indexes_names_and_localities() {
        let LocalityMap { locality_map, .. } = locality_map_fixture();
        let public_spaces = vec![
            PublicSpace {
                id: 2,
                name: "Spoorstraat".to_string(),
                locality_id: 10,
            },
            PublicSpace {
                id: 1,
                name: "Hoofdweg".to_string(),
                locality_id: 11,
            },
            PublicSpace {
                id: 3,
                name: "Spoorstraat".to_string(),
                locality_id: 12,
            },
        ];

        let (names, map) = index_public_spaces(public_spaces, locality_map);

        assert_eq!(names, vec!["Hoofdweg", "Spoorstraat"]);
        assert_eq!(map.get(&1), Some(&(0, 0)));
        assert_eq!(map.get(&2), Some(&(1, 2)));
        assert_eq!(map.get(&3), Some(&(1, 1)));
    }

    #[test]
    fn encode_addresses_groups_and_sorts_ranges() {
        let mut public_spaces_map = std::collections::HashMap::new();
        public_spaces_map.insert(1u64, (0, 0));
        public_spaces_map.insert(2u64, (1, 0));

        let addresses = vec![
            Address {
                house_number: 2,

                postal_code: "1234AB".to_string(),
                public_space_id: 1,
            },
            Address {
                house_number: 1,

                postal_code: "1234AB".to_string(),
                public_space_id: 1,
            },
            Address {
                house_number: 2,

                postal_code: "1234AB".to_string(),
                public_space_id: 1,
            },
            Address {
                house_number: 4,

                postal_code: "1234AB".to_string(),
                public_space_id: 1,
            },
            Address {
                house_number: 1,

                postal_code: "1234AB".to_string(),
                public_space_id: 2,
            },
            Address {
                house_number: 3,

                postal_code: "1234AC".to_string(),
                public_space_id: 1,
            },
            Address {
                house_number: 9,

                postal_code: "1234AB".to_string(),
                public_space_id: 999,
            },
        ];

        let ranges = encode_addresses(addresses, &public_spaces_map);

        let pc_ab = encode_pc(b"1234AB");
        let pc_ac = encode_pc(b"1234AC");
        let expected = [
            NumberRange {
                postal_code: pc_ab,
                start: 1,
                length: 1,
                public_space_index: 0,
                locality_index: 0,
                step: 1,
            },
            NumberRange {
                postal_code: pc_ab,
                start: 4,
                length: 0,
                public_space_index: 0,
                locality_index: 0,
                step: 1,
            },
            NumberRange {
                postal_code: pc_ab,
                start: 1,
                length: 0,
                public_space_index: 1,
                locality_index: 0,
                step: 1,
            },
            NumberRange {
                postal_code: pc_ac,
                start: 3,
                length: 0,
                public_space_index: 0,
                locality_index: 0,
                step: 1,
            },
        ];

        assert_eq!(ranges.len(), expected.len());
        for (actual, expected) in ranges.iter().zip(expected.iter()) {
            assert_eq!(actual.postal_code, expected.postal_code);
            assert_eq!(actual.start, expected.start);
            assert_eq!(actual.length, expected.length);
            assert_eq!(actual.step, expected.step);
            assert_eq!(actual.public_space_index, expected.public_space_index);
            assert_eq!(actual.locality_index, expected.locality_index);
        }
    }

    #[test]
    fn encode_addresses_detects_step() {
        let mut public_spaces_map = std::collections::HashMap::new();
        public_spaces_map.insert(1u64, (0, 0));

        // Odd numbers 1,3,5,7 and even numbers 2,4,6
        let addresses: Vec<Address> = [1, 3, 5, 7, 2, 4, 6]
            .into_iter()
            .map(|n| Address {
                house_number: n,
                postal_code: "5678CD".to_string(),
                public_space_id: 1,
            })
            .collect();

        let ranges = encode_addresses(addresses, &public_spaces_map);

        let pc = encode_pc(b"5678CD");

        // Sorted: 1,2,3,4,5,6,7
        // 1→2: step=1, 1→2→3: step=1, ..., 1→2→3→4→5→6→7: single range step=1
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].postal_code, pc);
        assert_eq!(ranges[0].start, 1);
        assert_eq!(ranges[0].length, 6);
        assert_eq!(ranges[0].step, 1);
    }

    #[test]
    fn encode_addresses_odd_even_stepping() {
        let mut public_spaces_map = std::collections::HashMap::new();
        public_spaces_map.insert(1u64, (0, 0));

        // Only odd numbers: 1,3,5,7,9
        let addresses: Vec<Address> = [1, 3, 5, 7, 9]
            .into_iter()
            .map(|n| Address {
                house_number: n,
                postal_code: "5678CD".to_string(),
                public_space_id: 1,
            })
            .collect();

        let ranges = encode_addresses(addresses, &public_spaces_map);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 1);
        assert_eq!(ranges[0].length, 4);
        assert_eq!(ranges[0].step, 2);
    }

    #[test]
    fn encode_addresses_step_break() {
        let mut public_spaces_map = std::collections::HashMap::new();
        public_spaces_map.insert(1u64, (0, 0));

        // 2,4,6 then 9 (breaks the step=2 pattern)
        let addresses: Vec<Address> = [2, 4, 6, 9]
            .into_iter()
            .map(|n| Address {
                house_number: n,
                postal_code: "5678CD".to_string(),
                public_space_id: 1,
            })
            .collect();

        let ranges = encode_addresses(addresses, &public_spaces_map);

        assert_eq!(ranges.len(), 2);
        // First range: 2,4,6 with step=2
        assert_eq!(ranges[0].start, 2);
        assert_eq!(ranges[0].length, 2);
        assert_eq!(ranges[0].step, 2);
        // Second range: 9 alone
        assert_eq!(ranges[1].start, 9);
        assert_eq!(ranges[1].length, 0);
        assert_eq!(ranges[1].step, 1);
    }
}
