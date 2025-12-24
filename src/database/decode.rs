#[cfg(feature = "compressed_database")]
use std::io::Read;

use crate::Database;

#[cfg(feature = "compressed_database")]
use crate::database::error::DatabaseError;

#[cfg(feature = "compressed_database")]
use super::{
    NumberRange,
    layout::{Header, validate_offsets_iter},
    rw::read_u32_reader,
};

#[cfg(feature = "compressed_database")]
use super::rw::{read_bytes, read_offsets, read_u16_reader};

impl Database {
    /// Decode a database from a binary reader.
    #[cfg(feature = "compressed_database")]
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> Result<Self, DatabaseError> {
        let header = Header::from_reader(&mut reader)?;

        let locality_offsets = read_offsets(&mut reader, header.locality_count as usize + 1)?;
        let locality_data_len =
            validate_offsets_iter(locality_offsets.iter().copied().map(Ok))? as usize;
        let expected_locality_data_offset = header.expected_locality_data_offset()?;
        if header.locality_data_offset != expected_locality_data_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let locality_data = read_bytes(&mut reader, locality_data_len)?;
        let localities = decode_names(&locality_offsets, &locality_data)?;

        let expected_public_space_offsets_offset =
            header.expected_public_space_offsets_offset(locality_data_len)?;
        if header.public_space_offsets_offset != expected_public_space_offsets_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let public_space_offsets =
            read_offsets(&mut reader, header.public_space_count as usize + 1)?;
        let public_space_data_len =
            validate_offsets_iter(public_space_offsets.iter().copied().map(Ok))? as usize;
        let expected_public_space_data_offset = header.expected_public_space_data_offset()?;
        if header.public_space_data_offset != expected_public_space_data_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let public_space_data = read_bytes(&mut reader, public_space_data_len)?;
        let public_spaces = decode_names(&public_space_offsets, &public_space_data)?;

        let expected_ranges_offset = header.expected_ranges_offset(public_space_data_len)?;
        if header.ranges_offset != expected_ranges_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let mut ranges = Vec::with_capacity(header.range_count as usize);
        for _ in 0..header.range_count {
            let postal_code = read_u32_reader(&mut reader)?;
            let start = read_u32_reader(&mut reader)?;
            let length = read_u16_reader(&mut reader)?;
            let public_space_index = read_u32_reader(&mut reader)?;
            let locality_index = read_u16_reader(&mut reader)?;

            ranges.push(NumberRange {
                postal_code,
                start,
                length,
                public_space_index,
                locality_index,
            });
        }

        Ok(Self {
            localities,
            public_spaces,
            ranges,
        })
    }

    /// Return true when there are no ranges loaded.
    pub(crate) fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub(crate) fn locality_name(&self, index: u16) -> Option<&str> {
        self.localities.get(index as usize).map(String::as_str)
    }

    pub(crate) fn public_space_name(&self, index: u32) -> Option<&str> {
        self.public_spaces.get(index as usize).map(String::as_str)
    }
}

#[cfg(feature = "compressed_database")]
fn decode_names(offsets: &[u32], data: &[u8]) -> Result<Vec<String>, DatabaseError> {
    if offsets.len() < 2 {
        return Err(DatabaseError::InvalidLayout);
    }
    let mut names = Vec::with_capacity(offsets.len() - 1);
    for window in offsets.windows(2) {
        let start = window[0] as usize;
        let end = window[1] as usize;
        if start > end || end > data.len() {
            return Err(DatabaseError::InvalidLayout);
        }
        let name =
            std::str::from_utf8(&data[start..end]).map_err(|_| DatabaseError::InvalidLayout)?;
        names.push(name.to_string());
    }
    Ok(names)
}
