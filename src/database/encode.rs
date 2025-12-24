use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

#[cfg(feature = "compressed_database")]
use flate2::{Compression, write::GzEncoder};

use crate::Database;

use super::util::{DATABASE_HEADER_SIZE, DATABASE_MAGIC};

impl Database {
    /// Serialize the database to a binary file (optionally compressed).
    pub fn encode(&self, path: &Path) -> io::Result<()> {
        let locality_count = u32::try_from(self.localities.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "locality count overflow"))?;
        let public_space_count = u32::try_from(self.public_spaces.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "public space count overflow")
        })?;
        let range_count = u32::try_from(self.ranges.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "range count overflow"))?;

        let file = File::create(path)?;

        #[cfg(feature = "compressed_database")]
        {
            let mut encoder = GzEncoder::new(file, Compression::default());
            self.write_database(
                &mut encoder,
                locality_count,
                public_space_count,
                range_count,
            )?;
            encoder.finish()?;
            Ok(())
        }

        #[cfg(not(feature = "compressed_database"))]
        {
            let mut writer = file;
            self.write_database(&mut writer, locality_count, public_space_count, range_count)
        }
    }

    pub(crate) fn write_database<W: Write>(
        &self,
        writer: &mut W,
        locality_count: u32,
        public_space_count: u32,
        range_count: u32,
    ) -> io::Result<()> {
        let locality_offsets_offset = DATABASE_HEADER_SIZE;
        let locality_offsets_len = (locality_count as usize + 1) * 4;
        let locality_data_offset = locality_offsets_offset + locality_offsets_len;
        let locality_data_len: usize = self.localities.iter().map(|name| name.len()).sum();

        let public_space_offsets_offset = locality_data_offset + locality_data_len;
        let public_space_offsets_len = (public_space_count as usize + 1) * 4;
        let public_space_data_offset = public_space_offsets_offset + public_space_offsets_len;
        let public_space_data_len: usize = self.public_spaces.iter().map(|name| name.len()).sum();

        let ranges_offset = public_space_data_offset + public_space_data_len;

        writer.write_all(&DATABASE_MAGIC)?;
        writer.write_all(&locality_count.to_le_bytes())?;
        writer.write_all(&public_space_count.to_le_bytes())?;
        writer.write_all(&range_count.to_le_bytes())?;
        writer.write_all(&(locality_offsets_offset as u32).to_le_bytes())?;
        writer.write_all(&(locality_data_offset as u32).to_le_bytes())?;
        writer.write_all(&(public_space_offsets_offset as u32).to_le_bytes())?;
        writer.write_all(&(public_space_data_offset as u32).to_le_bytes())?;
        writer.write_all(&(ranges_offset as u32).to_le_bytes())?;

        let mut offset = 0u32;
        writer.write_all(&offset.to_le_bytes())?;
        for name in &self.localities {
            offset = offset.saturating_add(name.len() as u32);
            writer.write_all(&offset.to_le_bytes())?;
        }

        for name in &self.localities {
            writer.write_all(name.as_bytes())?;
        }

        offset = 0;
        writer.write_all(&offset.to_le_bytes())?;
        for name in &self.public_spaces {
            offset = offset.saturating_add(name.len() as u32);
            writer.write_all(&offset.to_le_bytes())?;
        }

        for name in &self.public_spaces {
            writer.write_all(name.as_bytes())?;
        }

        for range in &self.ranges {
            writer.write_all(&range.postal_code.to_le_bytes())?;
            writer.write_all(&range.start.to_le_bytes())?;
            writer.write_all(&range.length.to_le_bytes())?;
            writer.write_all(&range.public_space_index.to_le_bytes())?;
            writer.write_all(&range.locality_index.to_le_bytes())?;
        }

        Ok(())
    }
}
