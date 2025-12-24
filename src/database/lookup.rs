use crate::database::{DatabaseView, util::partition_point_range};

use super::{
    Database,
    util::{encode_pc, normalize_postalcode},
};

impl<'a> DatabaseView<'a> {
    pub fn lookup(&self, postalcode: &str, house_number: u32) -> Option<(&str, &str)> {
        let normalized_postalcode = normalize_postalcode(postalcode)?;
        let pc_encoded = encode_pc(&normalized_postalcode);

        let range_count = self.range_count as usize;
        let start = partition_point_range(range_count, |idx| {
            self.range_postal_code(idx)
                .is_none_or(|code| code < pc_encoded)
        });
        let end = partition_point_range(range_count, |idx| {
            self.range_postal_code(idx)
                .is_none_or(|code| code <= pc_encoded)
        });

        for index in start..end {
            let range = self.range_at(index)?;
            let range_end = range.start.checked_add(range.length as u32)?;
            if house_number >= range.start && house_number <= range_end {
                let public_space = self.public_space_name(range.public_space_index)?;
                let locality = self.locality_name(range.locality_index)?;
                return Some((public_space, locality));
            }
        }

        None
    }
}

impl Database {
    pub(crate) fn lookup(&self, postalcode: &str, house_number: u32) -> Option<(&str, &str)> {
        let postalcode = normalize_postalcode(postalcode)?;
        let pc_encoded = encode_pc(&postalcode);

        let start = self.ranges.partition_point(|r| r.postal_code < pc_encoded);

        let end = self.ranges.partition_point(|r| r.postal_code <= pc_encoded);

        for index in start..end {
            let range = self.ranges.get(index)?;
            let Some(range_end) = range.start.checked_add(range.length as u32) else {
                continue;
            };

            if house_number >= range.start && house_number <= range_end {
                let public_space_name = self.public_space_name(range.public_space_index)?;
                let locality_name = self.locality_name(range.locality_index)?;
                return Some((public_space_name, locality_name));
            }
        }

        None
    }
}
