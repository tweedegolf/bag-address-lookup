use std::io::Read;

use super::error::DatabaseError;

pub(crate) fn read_u32_reader<R: Read>(reader: &mut R) -> Result<u32, DatabaseError> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|_| DatabaseError::DecompressionFailed)?;
    Ok(u32::from_le_bytes(buf))
}

pub(crate) fn read_u32_bytes(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

pub(crate) fn read_u16_bytes(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes(slice.try_into().ok()?))
}

#[cfg(feature = "compressed_database")]
pub(crate) fn read_u16_reader<R: Read>(reader: &mut R) -> Result<u16, DatabaseError> {
    let mut buf = [0u8; 2];
    reader
        .read_exact(&mut buf)
        .map_err(|_| DatabaseError::DecompressionFailed)?;
    Ok(u16::from_le_bytes(buf))
}

#[cfg(feature = "compressed_database")]
pub(crate) fn read_bytes<R: Read>(reader: &mut R, len: usize) -> Result<Vec<u8>, DatabaseError> {
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .map_err(|_| DatabaseError::DecompressionFailed)?;
    Ok(buf)
}

#[cfg(feature = "compressed_database")]
pub(crate) fn read_offsets<R: Read>(
    reader: &mut R,
    count: usize,
) -> Result<Vec<u32>, DatabaseError> {
    let mut offsets = Vec::with_capacity(count);
    for _ in 0..count {
        offsets.push(read_u32_reader(reader)?);
    }
    Ok(offsets)
}
