use crate::database::{DatabaseView, layout::Header};

use super::{
    error::DatabaseError,
    layout::{OffsetsBytesIter, validate_offsets_iter},
    rw::{read_u8_bytes, read_u16_bytes, read_u32_bytes},
};

const RANGE_RECORD_SIZE: usize = 17;

pub(crate) struct RangeRef {
    pub(crate) start: u32,
    pub(crate) length: u16,
    pub(crate) public_space_index: u32,
    pub(crate) locality_index: u16,
    pub(crate) step: u8,
}

impl DatabaseView {
    pub fn from_bytes(bytes: &'static [u8]) -> Result<Self, DatabaseError> {
        let header = Header::from_bytes(bytes)?;

        let locality_offsets_len = header.locality_offsets_len()?;
        let locality_offsets_end = header
            .locality_offsets_offset
            .checked_add(locality_offsets_len)
            .ok_or(DatabaseError::InvalidLayout)?;
        let expected_locality_data_offset = header.expected_locality_data_offset()?;

        if locality_offsets_end > bytes.len()
            || header.locality_data_offset != expected_locality_data_offset
        {
            return Err(DatabaseError::InvalidLayout);
        }

        let locality_offsets_count = header
            .locality_count
            .checked_add(1)
            .ok_or(DatabaseError::InvalidLayout)? as usize;
        let locality_data_len = validate_offsets_iter(OffsetsBytesIter::new(
            bytes,
            header.locality_offsets_offset,
            locality_offsets_count,
        ))? as usize;
        if locality_data_len == 0 && header.locality_count != 0 {
            return Err(DatabaseError::InvalidLayout);
        }

        let public_space_offsets_expected =
            header.expected_public_space_offsets_offset(locality_data_len)?;
        if header.public_space_offsets_offset != public_space_offsets_expected {
            return Err(DatabaseError::InvalidLayout);
        }

        let public_space_offsets_len = header.public_space_offsets_len()?;
        let public_space_offsets_end = header
            .public_space_offsets_offset
            .checked_add(public_space_offsets_len)
            .ok_or(DatabaseError::InvalidLayout)?;
        let expected_public_space_data_offset = header.expected_public_space_data_offset()?;

        if public_space_offsets_end > bytes.len()
            || header.public_space_data_offset != expected_public_space_data_offset
        {
            return Err(DatabaseError::InvalidLayout);
        }

        let public_space_offsets_count = header
            .public_space_count
            .checked_add(1)
            .ok_or(DatabaseError::InvalidLayout)? as usize;
        let public_space_data_len = validate_offsets_iter(OffsetsBytesIter::new(
            bytes,
            header.public_space_offsets_offset,
            public_space_offsets_count,
        ))? as usize;
        if public_space_data_len == 0 && header.public_space_count != 0 {
            return Err(DatabaseError::InvalidLayout);
        }

        let ranges_expected = header.expected_ranges_offset(public_space_data_len)?;
        if header.ranges_offset != ranges_expected {
            return Err(DatabaseError::InvalidLayout);
        }

        let ranges_len = (header.range_count as usize)
            .checked_mul(RANGE_RECORD_SIZE)
            .ok_or(DatabaseError::InvalidLayout)?;
        let ranges_end = header
            .ranges_offset
            .checked_add(ranges_len)
            .ok_or(DatabaseError::InvalidLayout)?;
        if ranges_end > bytes.len() {
            return Err(DatabaseError::InvalidLayout);
        }

        // Validate municipality string table
        let municipality_offsets_len = header.municipality_offsets_len()?;
        let municipality_offsets_end = header
            .municipality_offsets_offset
            .checked_add(municipality_offsets_len)
            .ok_or(DatabaseError::InvalidLayout)?;
        let expected_municipality_data_offset = header.expected_municipality_data_offset()?;

        if municipality_offsets_end > bytes.len()
            || header.municipality_data_offset != expected_municipality_data_offset
        {
            return Err(DatabaseError::InvalidLayout);
        }

        let municipality_offsets_count = header
            .municipality_count
            .checked_add(1)
            .ok_or(DatabaseError::InvalidLayout)? as usize;
        let municipality_data_len = validate_offsets_iter(OffsetsBytesIter::new(
            bytes,
            header.municipality_offsets_offset,
            municipality_offsets_count,
        ))? as usize;

        // Validate province string table
        let province_offsets_expected =
            header.expected_province_offsets_offset(municipality_data_len)?;
        if header.province_offsets_offset != province_offsets_expected {
            return Err(DatabaseError::InvalidLayout);
        }

        let province_offsets_len = header.province_offsets_len()?;
        let province_offsets_end = header
            .province_offsets_offset
            .checked_add(province_offsets_len)
            .ok_or(DatabaseError::InvalidLayout)?;
        let expected_province_data_offset = header.expected_province_data_offset()?;

        if province_offsets_end > bytes.len()
            || header.province_data_offset != expected_province_data_offset
        {
            return Err(DatabaseError::InvalidLayout);
        }

        let province_offsets_count = header
            .province_count
            .checked_add(1)
            .ok_or(DatabaseError::InvalidLayout)? as usize;
        let province_data_len = validate_offsets_iter(OffsetsBytesIter::new(
            bytes,
            header.province_offsets_offset,
            province_offsets_count,
        ))? as usize;

        // Validate locality-municipality map
        let expected_loc_muni_offset =
            header.expected_locality_municipality_map_offset(province_data_len)?;
        if header.locality_municipality_map_offset != expected_loc_muni_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        // Validate municipality-province map
        let expected_muni_prov_offset = header.expected_municipality_province_map_offset()?;
        if header.municipality_province_map_offset != expected_muni_prov_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        // Validate municipality codes
        let expected_codes_offset = header.expected_municipality_codes_offset()?;
        if header.municipality_codes_offset != expected_codes_offset {
            return Err(DatabaseError::InvalidLayout);
        }

        let municipality_codes_end = header
            .municipality_codes_offset
            .checked_add(
                (header.municipality_count as usize)
                    .checked_mul(2)
                    .ok_or(DatabaseError::InvalidLayout)?,
            )
            .ok_or(DatabaseError::InvalidLayout)?;
        if municipality_codes_end > bytes.len() {
            return Err(DatabaseError::InvalidLayout);
        }

        Ok(Self {
            bytes,
            locality_count: header.locality_count,
            public_space_count: header.public_space_count,
            range_count: header.range_count,
            locality_offsets_offset: header.locality_offsets_offset,
            locality_data_offset: header.locality_data_offset,
            locality_data_end: header.public_space_offsets_offset,
            public_space_offsets_offset: header.public_space_offsets_offset,
            public_space_data_offset: header.public_space_data_offset,
            public_space_data_end: header.ranges_offset,
            ranges_offset: header.ranges_offset,
            municipality_count: header.municipality_count,
            province_count: header.province_count,
            municipality_offsets_offset: header.municipality_offsets_offset,
            municipality_data_offset: header.municipality_data_offset,
            municipality_data_end: header.province_offsets_offset,
            province_offsets_offset: header.province_offsets_offset,
            province_data_offset: header.province_data_offset,
            province_data_end: header.locality_municipality_map_offset,
            locality_municipality_map_offset: header.locality_municipality_map_offset,
            municipality_province_map_offset: header.municipality_province_map_offset,
            municipality_codes_offset: header.municipality_codes_offset,
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.range_count == 0
    }

    pub(crate) fn range_postal_code(&self, index: usize) -> Option<u32> {
        let base = self.range_offset(index)?;
        read_u32_bytes(self.bytes, base)
    }

    pub(crate) fn range_at(&self, index: usize) -> Option<RangeRef> {
        let base = self.range_offset(index)?;
        Some(RangeRef {
            start: read_u32_bytes(self.bytes, base + 4)?,
            length: read_u16_bytes(self.bytes, base + 8)?,
            public_space_index: read_u32_bytes(self.bytes, base + 10)?,
            locality_index: read_u16_bytes(self.bytes, base + 14)?,
            step: read_u8_bytes(self.bytes, base + 16)?,
        })
    }

    fn range_offset(&self, index: usize) -> Option<usize> {
        let offset = index.checked_mul(RANGE_RECORD_SIZE)?;
        let base = self.ranges_offset.checked_add(offset)?;
        if base + RANGE_RECORD_SIZE <= self.bytes.len() {
            Some(base)
        } else {
            None
        }
    }

    pub(crate) fn locality_name(&self, index: u16) -> Option<&'static str> {
        self.name_at(
            self.locality_offsets_offset,
            self.locality_data_offset,
            self.locality_data_end,
            index as u32,
            self.locality_count,
        )
    }

    pub(crate) fn public_space_name(&self, index: u32) -> Option<&'static str> {
        self.name_at(
            self.public_space_offsets_offset,
            self.public_space_data_offset,
            self.public_space_data_end,
            index,
            self.public_space_count,
        )
    }

    pub(crate) fn municipality_name(&self, index: u16) -> Option<&'static str> {
        self.name_at(
            self.municipality_offsets_offset,
            self.municipality_data_offset,
            self.municipality_data_end,
            index as u32,
            self.municipality_count,
        )
    }

    pub(crate) fn province_name(&self, index: u8) -> Option<&'static str> {
        self.name_at(
            self.province_offsets_offset,
            self.province_data_offset,
            self.province_data_end,
            index as u32,
            self.province_count,
        )
    }

    pub(crate) fn locality_municipality_index(&self, locality_index: u16) -> Option<u16> {
        if (locality_index as u32) >= self.locality_count {
            return None;
        }
        read_u16_bytes(
            self.bytes,
            self.locality_municipality_map_offset + locality_index as usize * 2,
        )
    }

    pub(crate) fn municipality_province_index(&self, municipality_index: u16) -> Option<u8> {
        if (municipality_index as u32) >= self.municipality_count {
            return None;
        }
        read_u8_bytes(
            self.bytes,
            self.municipality_province_map_offset + municipality_index as usize,
        )
    }

    pub(crate) fn municipality_code(&self, municipality_index: u16) -> Option<u16> {
        if (municipality_index as u32) >= self.municipality_count {
            return None;
        }
        read_u16_bytes(
            self.bytes,
            self.municipality_codes_offset + municipality_index as usize * 2,
        )
    }

    pub(crate) fn locality_details(&self) -> Vec<(&'static str, &'static str, u16)> {
        let mut result = Vec::new();
        for i in 0..self.locality_count {
            let loc_idx = i as u16;
            let Some(name) = self.locality_name(loc_idx) else {
                continue;
            };
            let Some(m_idx) = self.locality_municipality_index(loc_idx) else {
                continue;
            };
            if m_idx == u16::MAX {
                continue;
            }
            let m_name = self.municipality_name(m_idx).unwrap_or("");
            let m_code = self.municipality_code(m_idx).unwrap_or(0);
            result.push((name, m_name, m_code));
        }
        result
    }

    pub(crate) fn municipality_details(&self) -> Vec<(&'static str, u16, &'static str)> {
        let mut result = Vec::new();
        for i in 0..self.municipality_count {
            let m_idx = i as u16;
            let Some(name) = self.municipality_name(m_idx) else {
                continue;
            };
            let code = self.municipality_code(m_idx).unwrap_or(0);
            let p_idx = self.municipality_province_index(m_idx).unwrap_or(u8::MAX);
            let p_name = self.province_name(p_idx).unwrap_or("");
            result.push((name, code, p_name));
        }
        result
    }

    fn name_at(
        &self,
        offsets_offset: usize,
        data_offset: usize,
        data_end: usize,
        index: u32,
        count: u32,
    ) -> Option<&'static str> {
        if index >= count {
            return None;
        }

        let start = read_u32_bytes(self.bytes, offsets_offset + index as usize * 4)? as usize;
        let end = read_u32_bytes(self.bytes, offsets_offset + (index as usize + 1) * 4)? as usize;
        if start > end {
            return None;
        }

        let start_abs = data_offset.checked_add(start)?;
        let end_abs = data_offset.checked_add(end)?;
        if end_abs > data_end || start_abs > end_abs {
            return None;
        }

        std::str::from_utf8(self.bytes.get(start_abs..end_abs)?).ok()
    }
}
