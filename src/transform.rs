use std::{collections::HashMap, error::Error};

use crate::{Address, Locality, NumberRange, PublicSpace, encode_pc};

pub struct LocalityMap {
    pub locality_names: Vec<String>,
    pub locality_map: HashMap<u16, u16>,
}

/// Build stable locality name indexes and id mappings.
pub fn index_localities(localities: Vec<Locality>) -> Result<LocalityMap, Box<dyn Error>> {
    let mut locality_names: Vec<String> = localities
        .iter()
        .map(|locality| locality.name.clone())
        .collect();
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
    for locality in localities {
        if let Some(index) = name_index.get(&locality.name) {
            locality_map.insert(locality.id, *index);
        }
    }

    Ok(LocalityMap {
        locality_names,
        locality_map,
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
                let range_end = range.start + range.length as u32;
                if house_number <= range_end {
                    continue;
                }

                if range.length < u16::MAX && house_number == range_end + 1 {
                    range.length = range.length.saturating_add(1);
                } else {
                    let finished = current.take().expect("Range is missing");
                    ranges.push(finished);
                    current = Some(NumberRange {
                        postal_code,
                        start: house_number,
                        length: 0,
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

        index_localities(localities).expect("locality map fixture")
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

        let result = index_localities(localities);

        assert!(result.is_err());
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
                house_letter: None,
                house_number_addition: None,
                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-2".to_string(),
                house_number: 1,
                house_letter: None,
                house_number_addition: None,
                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-3".to_string(),
                house_number: 2,
                house_letter: None,
                house_number_addition: None,
                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-4".to_string(),
                house_number: 4,
                house_letter: None,
                house_number_addition: None,
                postal_code: "1234AB".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-5".to_string(),
                house_number: 1,
                house_letter: None,
                house_number_addition: None,
                postal_code: "1234AB".to_string(),
                public_space_id: "ps-2".to_string(),
            },
            Address {
                id: "a-6".to_string(),
                house_number: 3,
                house_letter: None,
                house_number_addition: None,
                postal_code: "1234AC".to_string(),
                public_space_id: "ps-1".to_string(),
            },
            Address {
                id: "a-7".to_string(),
                house_number: 9,
                house_letter: None,
                house_number_addition: None,
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
            },
            NumberRange {
                postal_code: pc_ab,
                start: 4,
                length: 0,
                public_space_index: 0,
                locality_index: 0,
            },
            NumberRange {
                postal_code: pc_ab,
                start: 1,
                length: 0,
                public_space_index: 1,
                locality_index: 0,
            },
            NumberRange {
                postal_code: pc_ac,
                start: 3,
                length: 0,
                public_space_index: 0,
                locality_index: 0,
            },
        ];

        assert_eq!(ranges.len(), expected.len());
        for (actual, expected) in ranges.iter().zip(expected.iter()) {
            assert_eq!(actual.postal_code, expected.postal_code);
            assert_eq!(actual.start, expected.start);
            assert_eq!(actual.length, expected.length);
            assert_eq!(actual.public_space_index, expected.public_space_index);
            assert_eq!(actual.locality_index, expected.locality_index);
        }
    }
}
