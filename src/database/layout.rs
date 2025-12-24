use std::io::{Cursor, Read};

use crate::database::error::DatabaseError;

use super::{
    rw::{read_u32_bytes, read_u32_reader},
    util::{DATABASE_HEADER_SIZE, DATABASE_MAGIC},
};

pub(crate) struct Header {
    pub(crate) locality_count: u32,
    pub(crate) public_space_count: u32,
    pub(crate) range_count: u32,
    pub(crate) locality_offsets_offset: usize,
    pub(crate) locality_data_offset: usize,
    pub(crate) public_space_offsets_offset: usize,
    pub(crate) public_space_data_offset: usize,
    pub(crate) ranges_offset: usize,
}

impl Header {
    pub(crate) fn validate_base(&self) -> Result<(), DatabaseError> {
        if self.locality_offsets_offset != DATABASE_HEADER_SIZE {
            return Err(DatabaseError::InvalidLayout);
        }
        Ok(())
    }

    pub(crate) fn locality_offsets_len(&self) -> Result<usize, DatabaseError> {
        (self.locality_count as usize + 1)
            .checked_mul(4)
            .ok_or(DatabaseError::InvalidLayout)
    }

    pub(crate) fn public_space_offsets_len(&self) -> Result<usize, DatabaseError> {
        (self.public_space_count as usize + 1)
            .checked_mul(4)
            .ok_or(DatabaseError::InvalidLayout)
    }

    pub(crate) fn expected_locality_data_offset(&self) -> Result<usize, DatabaseError> {
        let offsets_len = self.locality_offsets_len()?;
        self.locality_offsets_offset
            .checked_add(offsets_len)
            .ok_or(DatabaseError::InvalidLayout)
    }

    pub(crate) fn expected_public_space_offsets_offset(
        &self,
        locality_data_len: usize,
    ) -> Result<usize, DatabaseError> {
        self.locality_data_offset
            .checked_add(locality_data_len)
            .ok_or(DatabaseError::InvalidLayout)
    }

    pub(crate) fn expected_public_space_data_offset(&self) -> Result<usize, DatabaseError> {
        let offsets_len = self.public_space_offsets_len()?;
        self.public_space_offsets_offset
            .checked_add(offsets_len)
            .ok_or(DatabaseError::InvalidLayout)
    }

    pub(crate) fn expected_ranges_offset(
        &self,
        public_space_data_len: usize,
    ) -> Result<usize, DatabaseError> {
        self.public_space_data_offset
            .checked_add(public_space_data_len)
            .ok_or(DatabaseError::InvalidLayout)
    }

    pub(crate) fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DatabaseError> {
        let mut magic = [0u8; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|_| DatabaseError::DecompressionFailed)?;
        if magic != DATABASE_MAGIC {
            return Err(DatabaseError::InvalidMagic);
        }

        let locality_count = read_u32_reader(reader)?;
        let public_space_count = read_u32_reader(reader)?;
        let range_count = read_u32_reader(reader)?;

        let locality_offsets_offset = read_u32_reader(reader)? as usize;
        let locality_data_offset = read_u32_reader(reader)? as usize;
        let public_space_offsets_offset = read_u32_reader(reader)? as usize;
        let public_space_data_offset = read_u32_reader(reader)? as usize;
        let ranges_offset = read_u32_reader(reader)? as usize;

        let header = Self {
            locality_count,
            public_space_count,
            range_count,
            locality_offsets_offset,
            locality_data_offset,
            public_space_offsets_offset,
            public_space_data_offset,
            ranges_offset,
        };

        header.validate_base()?;
        Ok(header)
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Header, DatabaseError> {
        if bytes.len() < DATABASE_HEADER_SIZE {
            return Err(DatabaseError::TooShort);
        }
        let mut cursor = Cursor::new(bytes);
        Header::from_reader(&mut cursor)
    }
}

pub(crate) fn validate_offsets_iter<I>(iter: I) -> Result<u32, DatabaseError>
where
    I: IntoIterator<Item = Result<u32, DatabaseError>>,
{
    let mut iter = iter.into_iter();
    let first = iter
        .next()
        .transpose()?
        .ok_or(DatabaseError::InvalidLayout)?;
    if first != 0 {
        return Err(DatabaseError::InvalidLayout);
    }

    let mut prev = first;
    for value in iter {
        let value = value?;
        if value < prev {
            return Err(DatabaseError::InvalidLayout);
        }
        prev = value;
    }
    Ok(prev)
}

pub(crate) struct OffsetsBytesIter<'a> {
    bytes: &'a [u8],
    base: usize,
    count: usize,
    index: usize,
}

impl<'a> OffsetsBytesIter<'a> {
    pub(crate) fn new(bytes: &'a [u8], base: usize, count: usize) -> Self {
        Self {
            bytes,
            base,
            count,
            index: 0,
        }
    }
}

impl<'a> Iterator for OffsetsBytesIter<'a> {
    type Item = Result<u32, DatabaseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let index = self.index;
        self.index += 1;

        let offset = match index
            .checked_mul(4)
            .and_then(|delta| self.base.checked_add(delta))
        {
            Some(offset) => offset,
            None => return Some(Err(DatabaseError::InvalidLayout)),
        };

        match read_u32_bytes(self.bytes, offset) {
            Some(value) => Some(Ok(value)),
            None => Some(Err(DatabaseError::TooShort)),
        }
    }
}
