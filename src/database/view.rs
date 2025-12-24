use crate::database::{DatabaseView, layout::Header};

use super::{
    error::DatabaseError,
    layout::{OffsetsBytesIter, validate_offsets_iter},
    rw::{read_u16_bytes, read_u32_bytes},
};

const RANGE_RECORD_SIZE: usize = 16;

pub(crate) struct RangeRef {
    pub(crate) start: u32,
    pub(crate) length: u16,
    pub(crate) public_space_index: u32,
    pub(crate) locality_index: u16,
}

impl<'a> DatabaseView<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, DatabaseError> {
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

    pub(crate) fn locality_name(&self, index: u16) -> Option<&'a str> {
        self.name_at(
            self.locality_offsets_offset,
            self.locality_data_offset,
            self.locality_data_end,
            index as u32,
            self.locality_count,
        )
    }

    pub(crate) fn public_space_name(&self, index: u32) -> Option<&'a str> {
        self.name_at(
            self.public_space_offsets_offset,
            self.public_space_data_offset,
            self.public_space_data_end,
            index,
            self.public_space_count,
        )
    }

    fn name_at(
        &self,
        offsets_offset: usize,
        data_offset: usize,
        data_end: usize,
        index: u32,
        count: u32,
    ) -> Option<&'a str> {
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
