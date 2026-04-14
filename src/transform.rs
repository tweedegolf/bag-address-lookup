use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::{
    Address, Locality, NumberRange, PublicSpace, encode_pc,
    parsing::{MunicipalityRelation, municipalities::Municipality},
};

pub struct LocalityMap {
    pub locality_names: Vec<String>,
    pub locality_map: HashMap<u16, u16>,
}

/// Build a `locality_id -> province_name` map from GWR relations and CBS data.
pub fn build_locality_province_map<'a>(
    relations: &[MunicipalityRelation],
    cbs_municipalities: &'a [Municipality],
) -> HashMap<u16, &'a str> {
    let mut cbs: HashMap<u16, &str> = HashMap::with_capacity(cbs_municipalities.len());
    for m in cbs_municipalities {
        cbs.insert(m.code, m.province.as_str());
    }

    let mut result = HashMap::with_capacity(relations.len());
    for rel in relations {
        if let Some(&prov) = cbs.get(&rel.municipality_code) {
            result.insert(rel.locality_id, prov);
        }
    }
    result
}

/// Standard two-letter abbreviation for each Dutch province.
/// Panics on unrecognized province names so future additions fail loudly at build time.
pub fn province_abbreviation(province: &str) -> &'static str {
    match province {
        "Drenthe" => "DR",
        "Flevoland" => "FL",
        "Friesland" | "Fryslân" => "FR",
        "Gelderland" => "GE",
        "Groningen" => "GR",
        "Limburg" => "LI",
        "Noord-Brabant" => "NB",
        "Noord-Holland" => "NH",
        "Overijssel" => "OV",
        "Utrecht" => "UT",
        "Zeeland" => "ZE",
        "Zuid-Holland" => "ZH",
        other => panic!("Unknown province: {other}"),
    }
}

/// Build stable locality name indexes and id mappings.
///
/// Locality names that occur in more than one province are disambiguated by
/// appending the two-letter province code, e.g. `Hengelo (OV)` / `Hengelo (GE)`.
/// Names that are unique across provinces are left untouched.
pub fn index_localities(
    localities: Vec<Locality>,
    locality_province: &HashMap<u16, &str>,
) -> Result<LocalityMap, Box<dyn Error>> {
    // For each name, collect the distinct known provinces it occurs in.
    let mut name_provinces: HashMap<&str, HashSet<&str>> = HashMap::new();
    for locality in &localities {
        if let Some(&prov) = locality_province.get(&locality.id) {
            name_provinces
                .entry(locality.name.as_str())
                .or_default()
                .insert(prov);
        }
    }

    let display_name = |locality: &Locality| -> String {
        match locality_province.get(&locality.id) {
            Some(prov)
                if name_provinces
                    .get(locality.name.as_str())
                    .is_some_and(|set| set.len() > 1) =>
            {
                format!("{} ({})", locality.name, province_abbreviation(prov))
            }
            _ => locality.name.clone(),
        }
    };

    let mut locality_names: Vec<String> = localities.iter().map(display_name).collect();
    locality_names.sort();
    locality_names.dedup();

    if locality_names.len() > u16::MAX as usize {
        return Err("too many localities for u16 index".into());
    }

    let mut name_index = HashMap::with_capacity(locality_names.len());
    for (index, name) in locality_names.iter().enumerate() {
        name_index.insert(name.clone(), index as u16);
    }

    let mut locality_map = HashMap::with_capacity(localities.len());
    for locality in &localities {
        let name = display_name(locality);
        if let Some(index) = name_index.get(&name) {
            locality_map.insert(locality.id, *index);
        }
    }

    Ok(LocalityMap {
        locality_names,
        locality_map,
    })
}

pub struct MunicipalityMap {
    pub municipality_names: Vec<String>,
    pub province_names: Vec<String>,
    pub municipality_codes: Vec<u16>,
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
    // Build CBS lookup: gemeente_code -> (name, province)
    let mut cbs_lookup: HashMap<u16, (&str, &str)> =
        HashMap::with_capacity(cbs_municipalities.len());
    for m in cbs_municipalities {
        cbs_lookup.insert(m.code, (&m.name, &m.province));
    }

    // Build locality_id -> (municipality_name, province_name) from GWR + CBS
    let mut locality_to_municipality: HashMap<u16, (&str, &str)> =
        HashMap::with_capacity(relations.len());
    for rel in &relations {
        if let Some(&(name, province)) = cbs_lookup.get(&rel.municipality_code) {
            locality_to_municipality.insert(rel.locality_id, (name, province));
        }
    }

    // Collect and deduplicate municipality names (sorted)
    let mut municipality_names: Vec<String> = locality_to_municipality
        .values()
        .map(|(name, _)| name.to_string())
        .collect();
    municipality_names.sort();
    municipality_names.dedup();

    if municipality_names.len() > u16::MAX as usize {
        return Err("too many municipalities for u16 index".into());
    }

    let mut municipality_name_index: HashMap<&str, u16> =
        HashMap::with_capacity(municipality_names.len());
    for (i, name) in municipality_names.iter().enumerate() {
        municipality_name_index.insert(name, i as u16);
    }

    // Collect and deduplicate province names (sorted)
    let mut province_names: Vec<String> = locality_to_municipality
        .values()
        .map(|(_, prov)| prov.to_string())
        .collect();
    province_names.sort();
    province_names.dedup();

    if province_names.len() > u8::MAX as usize {
        return Err("too many provinces for u8 index".into());
    }

    let mut province_name_index: HashMap<&str, u8> = HashMap::with_capacity(province_names.len());
    for (i, name) in province_names.iter().enumerate() {
        province_name_index.insert(name, i as u8);
    }

    // Build municipality_codes: for each municipality_index, the CBS code
    let mut municipality_codes = vec![0u16; municipality_names.len()];
    for m in cbs_municipalities {
        if let Some(&idx) = municipality_name_index.get(m.name.as_str()) {
            municipality_codes[idx as usize] = m.code;
        }
    }

    // Build municipality_province: for each municipality_index, the province_index
    let mut municipality_province = vec![0u8; municipality_names.len()];
    for m in cbs_municipalities {
        if let Some(&m_idx) = municipality_name_index.get(m.name.as_str()) {
            if let Some(&p_idx) = province_name_index.get(m.province.as_str()) {
                municipality_province[m_idx as usize] = p_idx;
            }
        }
    }

    // Build locality_municipality: for each locality_index, the municipality_index
    // Use u16::MAX as sentinel for localities without a known municipality
    let mut locality_municipality = vec![u16::MAX; locality_count];
    for (locality_id, &locality_index) in locality_map {
        if let Some(&(muni_name, _)) = locality_to_municipality.get(locality_id) {
            if let Some(&m_idx) = municipality_name_index.get(muni_name) {
                locality_municipality[locality_index as usize] = m_idx;
            }
        }
    }

    Ok(MunicipalityMap {
        municipality_names,
        province_names,
        municipality_codes,
        locality_municipality,
        municipality_province,
    })
}

/// Build public space name indexes and map ids to locality indexes.
pub fn index_public_spaces(
    public_spaces: Vec<PublicSpace>,
    locality_map: HashMap<u16, u16>,
) -> (Vec<String>, HashMap<String, (u32, u16)>) {
    let mut public_space_names: Vec<String> = public_spaces
        .iter()
        .map(|public_space| public_space.name.clone())
        .collect();
    public_space_names.sort();
    public_space_names.dedup();

    let mut name_index = HashMap::with_capacity(public_space_names.len());
    for (index, name) in public_space_names.iter().enumerate() {
        name_index.insert(name.clone(), index as u32);
    }

    let mut public_spaces_map = HashMap::with_capacity(public_spaces.len());
    for public_space in public_spaces {
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
    public_spaces_map: &HashMap<String, (u32, u16)>,
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
    use std::collections::HashMap;

    use super::{LocalityMap, encode_addresses, index_localities, index_public_spaces};
    use crate::{Address, Locality, NumberRange, PublicSpace, encode_pc};

    fn locality_map_fixture() -> LocalityMap {
        let localities = vec![
            Locality {
                id: 10,
                name: "Beta".to_string(),
            },
            Locality {
                id: 11,
                name: "Alpha".to_string(),
            },
            Locality {
                id: 12,
                name: "Alpha".to_string(),
            },
        ];

        index_localities(localities, &HashMap::new()).expect("locality map fixture")
    }

    #[test]
    fn index_localities_sorts_and_dedups() {
        let result = locality_map_fixture();

        assert_eq!(result.locality_names, vec!["Alpha", "Beta"]);
        assert_eq!(result.locality_map.get(&10), Some(&1));
        assert_eq!(result.locality_map.get(&11), Some(&0));
        assert_eq!(result.locality_map.get(&12), Some(&0));
    }

    #[test]
    fn index_localities_rejects_overflow() {
        let localities = (0..=u16::MAX)
            .map(|id| Locality {
                id,
                name: format!("L{id}"),
            })
            .collect();

        let result = index_localities(localities, &HashMap::new());

        assert!(result.is_err());
    }

    #[test]
    fn index_localities_disambiguates_across_provinces() {
        let localities = vec![
            Locality {
                id: 1,
                name: "Hengelo".to_string(),
            },
            Locality {
                id: 2,
                name: "Hengelo".to_string(),
            },
            Locality {
                id: 3,
                name: "Utrecht".to_string(),
            },
        ];
        let mut provinces = HashMap::new();
        provinces.insert(1u16, "Overijssel");
        provinces.insert(2u16, "Gelderland");
        provinces.insert(3u16, "Utrecht");

        let result = index_localities(localities, &provinces).expect("disambiguation");

        assert_eq!(
            result.locality_names,
            vec!["Hengelo (GE)", "Hengelo (OV)", "Utrecht"]
        );
        assert_eq!(result.locality_map.get(&1), Some(&1));
        assert_eq!(result.locality_map.get(&2), Some(&0));
        assert_eq!(result.locality_map.get(&3), Some(&2));
    }

    #[test]
    fn index_localities_dedups_within_same_province() {
        let localities = vec![
            Locality {
                id: 1,
                name: "Hengelo".to_string(),
            },
            Locality {
                id: 2,
                name: "Hengelo".to_string(),
            },
        ];
        let mut provinces = HashMap::new();
        provinces.insert(1u16, "Overijssel");
        provinces.insert(2u16, "Overijssel");

        let result = index_localities(localities, &provinces).expect("same-province dedup");

        assert_eq!(result.locality_names, vec!["Hengelo"]);
        assert_eq!(result.locality_map.get(&1), Some(&0));
        assert_eq!(result.locality_map.get(&2), Some(&0));
    }

    #[test]
    fn index_public_spaces_indexes_names_and_localities() {
        let LocalityMap { locality_map, .. } = locality_map_fixture();
        let public_spaces = vec![
            PublicSpace {
                id: "ps-2".to_string(),
                name: "Spoorstraat".to_string(),
                locality_id: 10,
            },
            PublicSpace {
                id: "ps-1".to_string(),
                name: "Hoofdweg".to_string(),
                locality_id: 11,
            },
            PublicSpace {
                id: "ps-3".to_string(),
                name: "Spoorstraat".to_string(),
                locality_id: 12,
            },
        ];

        let (names, map) = index_public_spaces(public_spaces, locality_map);

        assert_eq!(names, vec!["Hoofdweg", "Spoorstraat"]);
        assert_eq!(map.get("ps-1"), Some(&(0, 0)));
        assert_eq!(map.get("ps-2"), Some(&(1, 1)));
        assert_eq!(map.get("ps-3"), Some(&(1, 0)));
    }

    #[test]
    fn encode_addresses_groups_and_sorts_ranges() {
        let mut public_spaces_map = std::collections::HashMap::new();
        public_spaces_map.insert("ps-1".to_string(), (0, 0));
        public_spaces_map.insert("ps-2".to_string(), (1, 0));

        let addresses = vec![
            Address {
                id: "a-1".to_string(),
                house_number: 2,

                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-2".to_string(),
                house_number: 1,

                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-3".to_string(),
                house_number: 2,

                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-4".to_string(),
                house_number: 4,

                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-5".to_string(),
                house_number: 1,

                postal_code: "1234AB".to_string(),
                public_space_id: "ps-2".to_string(),
            },
            Address {
                id: "a-6".to_string(),
                house_number: 3,

                postal_code: "1234AC".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-7".to_string(),
                house_number: 9,

                postal_code: "1234AB".to_string(),
                public_space_id: "missing".to_string(),
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
        public_spaces_map.insert("ps-1".to_string(), (0, 0));

        // Odd numbers 1,3,5,7 and even numbers 2,4,6
        let addresses: Vec<Address> = [1, 3, 5, 7, 2, 4, 6]
            .into_iter()
            .enumerate()
            .map(|(i, n)| Address {
                id: format!("a-{i}"),
                house_number: n,
                postal_code: "5678CD".to_string(),
                public_space_id: "ps-1".to_string(),
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
        public_spaces_map.insert("ps-1".to_string(), (0, 0));

        // Only odd numbers: 1,3,5,7,9
        let addresses: Vec<Address> = [1, 3, 5, 7, 9]
            .into_iter()
            .enumerate()
            .map(|(i, n)| Address {
                id: format!("a-{i}"),
                house_number: n,
                postal_code: "5678CD".to_string(),
                public_space_id: "ps-1".to_string(),
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
        public_spaces_map.insert("ps-1".to_string(), (0, 0));

        // 2,4,6 then 9 (breaks the step=2 pattern)
        let addresses: Vec<Address> = [2, 4, 6, 9]
            .into_iter()
            .enumerate()
            .map(|(i, n)| Address {
                id: format!("a-{i}"),
                house_number: n,
                postal_code: "5678CD".to_string(),
                public_space_id: "ps-1".to_string(),
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
