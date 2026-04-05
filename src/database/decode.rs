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
use super::rw::{read_bytes, read_offsets, read_u8_reader, read_u16_reader};

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
            let step = read_u8_reader(&mut reader)?;

            ranges.push(NumberRange {
                postal_code,
                start,
                length,
                public_space_index,
                locality_index,
                step,
            });
        }

        // Decode municipality string table
        let municipality_offsets =
            read_offsets(&mut reader, header.municipality_count as usize + 1)?;
        let municipality_data_len =
            validate_offsets_iter(municipality_offsets.iter().copied().map(Ok))? as usize;
        let expected_municipality_data_offset = header.expected_municipality_data_offset()?;
        if header.municipality_data_offset != expected_municipality_data_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let municipality_data = read_bytes(&mut reader, municipality_data_len)?;
        let municipalities = if header.municipality_count == 0 {
            Vec::new()
        } else {
            decode_names(&municipality_offsets, &municipality_data)?
        };

        // Decode province string table
        let expected_province_offsets_offset =
            header.expected_province_offsets_offset(municipality_data_len)?;
        if header.province_offsets_offset != expected_province_offsets_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let province_offsets = read_offsets(&mut reader, header.province_count as usize + 1)?;
        let province_data_len =
            validate_offsets_iter(province_offsets.iter().copied().map(Ok))? as usize;
        let expected_province_data_offset = header.expected_province_data_offset()?;
        if header.province_data_offset != expected_province_data_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let province_data = read_bytes(&mut reader, province_data_len)?;
        let provinces = if header.province_count == 0 {
            Vec::new()
        } else {
            decode_names(&province_offsets, &province_data)?
        };

        // Decode locality -> municipality index map
        let expected_loc_muni_offset =
            header.expected_locality_municipality_map_offset(province_data_len)?;
        if header.locality_municipality_map_offset != expected_loc_muni_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let mut locality_municipality = Vec::with_capacity(header.locality_count as usize);
        for _ in 0..header.locality_count {
            locality_municipality.push(read_u16_reader(&mut reader)?);
        }

        // Decode municipality -> province index map
        let expected_muni_prov_offset = header.expected_municipality_province_map_offset()?;
        if header.municipality_province_map_offset != expected_muni_prov_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let mut municipality_province = Vec::with_capacity(header.municipality_count as usize);
        for _ in 0..header.municipality_count {
            municipality_province.push(read_u8_reader(&mut reader)?);
        }

        // Decode municipality codes
        let expected_codes_offset = header.expected_municipality_codes_offset()?;
        if header.municipality_codes_offset != expected_codes_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let mut municipality_codes = Vec::with_capacity(header.municipality_count as usize);
        for _ in 0..header.municipality_count {
            municipality_codes.push(read_u16_reader(&mut reader)?);
        }

        Ok(Self {
            localities,
            public_spaces,
            ranges,
            municipalities,
            provinces,
            municipality_codes,
            locality_municipality,
            municipality_province,
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

    pub(crate) fn municipality_name(&self, index: u16) -> Option<&str> {
        self.municipalities.get(index as usize).map(String::as_str)
    }

    pub(crate) fn province_name(&self, index: u8) -> Option<&str> {
        self.provinces.get(index as usize).map(String::as_str)
    }

    pub(crate) fn locality_details(&self) -> Vec<(&str, &str, u16)> {
        let mut result = Vec::with_capacity(self.localities.len());
        for (i, name) in self.localities.iter().enumerate() {
            let m_idx = self
                .locality_municipality
                .get(i)
                .copied()
                .unwrap_or(u16::MAX);
            if m_idx == u16::MAX {
                continue;
            }
            let m_name = self.municipality_name(m_idx).unwrap_or("");
            let m_code = self
                .municipality_codes
                .get(m_idx as usize)
                .copied()
                .unwrap_or(0);
            result.push((name.as_str(), m_name, m_code));
        }
        result
    }

    pub(crate) fn municipality_details(&self) -> Vec<(&str, u16, &str)> {
        let mut result = Vec::with_capacity(self.municipalities.len());
        for (i, name) in self.municipalities.iter().enumerate() {
            let code = self.municipality_codes.get(i).copied().unwrap_or(0);
            let p_idx = self
                .municipality_province
                .get(i)
                .copied()
                .unwrap_or(u8::MAX);
            let p_name = self.province_name(p_idx).unwrap_or("");
            result.push((name.as_str(), code, p_name));
        }
        result
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
