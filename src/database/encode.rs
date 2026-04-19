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
        let municipality_count = u32::try_from(self.municipalities.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "municipality count overflow")
        })?;
        let province_count = u32::try_from(self.provinces.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "province count overflow"))?;

        // Existing section offsets
        let locality_offsets_offset = DATABASE_HEADER_SIZE;
        let locality_offsets_len = (locality_count as usize + 1) * 4;
        let locality_data_offset = locality_offsets_offset + locality_offsets_len;
        let locality_data_len: usize = self.localities.iter().map(|name| name.len()).sum();

        let public_space_offsets_offset = locality_data_offset + locality_data_len;
        let public_space_offsets_len = (public_space_count as usize + 1) * 4;
        let public_space_data_offset = public_space_offsets_offset + public_space_offsets_len;
        let public_space_data_len: usize = self.public_spaces.iter().map(|name| name.len()).sum();

        let ranges_offset = public_space_data_offset + public_space_data_len;
        let range_record_size = 17; // 4+4+2+4+2+1
        let ranges_len = range_count as usize * range_record_size;

        // New section offsets
        let municipality_offsets_offset = ranges_offset + ranges_len;
        let municipality_offsets_len = (municipality_count as usize + 1) * 4;
        let municipality_data_offset = municipality_offsets_offset + municipality_offsets_len;
        let municipality_data_len: usize = self.municipalities.iter().map(|name| name.len()).sum();

        let province_offsets_offset = municipality_data_offset + municipality_data_len;
        let province_offsets_len = (province_count as usize + 1) * 4;
        let province_data_offset = province_offsets_offset + province_offsets_len;
        let province_data_len: usize = self.provinces.iter().map(|name| name.len()).sum();

        let locality_municipality_map_offset = province_data_offset + province_data_len;
        let locality_municipality_map_len = locality_count as usize * 2;

        let municipality_province_map_offset =
            locality_municipality_map_offset + locality_municipality_map_len;
        let municipality_province_map_len = municipality_count as usize;

        let municipality_codes_offset =
            municipality_province_map_offset + municipality_province_map_len;
        let municipality_codes_len = municipality_count as usize * 2;

        let locality_codes_offset = municipality_codes_offset + municipality_codes_len;
        let locality_codes_len = locality_count as usize * 2;

        let locality_had_suffix_offset = locality_codes_offset + locality_codes_len;
        let locality_had_suffix_len = locality_count as usize;

        let municipality_had_suffix_offset = locality_had_suffix_offset + locality_had_suffix_len;

        // Write header
        writer.write_all(&DATABASE_MAGIC)?;
        writer.write_all(&locality_count.to_le_bytes())?;
        writer.write_all(&public_space_count.to_le_bytes())?;
        writer.write_all(&range_count.to_le_bytes())?;
        writer.write_all(&(locality_offsets_offset as u32).to_le_bytes())?;
        writer.write_all(&(locality_data_offset as u32).to_le_bytes())?;
        writer.write_all(&(public_space_offsets_offset as u32).to_le_bytes())?;
        writer.write_all(&(public_space_data_offset as u32).to_le_bytes())?;
        writer.write_all(&(ranges_offset as u32).to_le_bytes())?;
        writer.write_all(&municipality_count.to_le_bytes())?;
        writer.write_all(&province_count.to_le_bytes())?;
        writer.write_all(&(municipality_offsets_offset as u32).to_le_bytes())?;
        writer.write_all(&(municipality_data_offset as u32).to_le_bytes())?;
        writer.write_all(&(province_offsets_offset as u32).to_le_bytes())?;
        writer.write_all(&(province_data_offset as u32).to_le_bytes())?;
        writer.write_all(&(locality_municipality_map_offset as u32).to_le_bytes())?;
        writer.write_all(&(municipality_province_map_offset as u32).to_le_bytes())?;
        writer.write_all(&(municipality_codes_offset as u32).to_le_bytes())?;
        writer.write_all(&(locality_codes_offset as u32).to_le_bytes())?;
        writer.write_all(&(locality_had_suffix_offset as u32).to_le_bytes())?;
        writer.write_all(&(municipality_had_suffix_offset as u32).to_le_bytes())?;

        // Write locality string table
        let mut offset = 0u32;
        writer.write_all(&offset.to_le_bytes())?;
        for name in &self.localities {
            offset = offset.saturating_add(name.len() as u32);
            writer.write_all(&offset.to_le_bytes())?;
        }
        for name in &self.localities {
            writer.write_all(name.as_bytes())?;
        }

        // Write public space string table
        offset = 0;
        writer.write_all(&offset.to_le_bytes())?;
        for name in &self.public_spaces {
            offset = offset.saturating_add(name.len() as u32);
            writer.write_all(&offset.to_le_bytes())?;
        }
        for name in &self.public_spaces {
            writer.write_all(name.as_bytes())?;
        }

        // Write ranges
        for range in &self.ranges {
            writer.write_all(&range.postal_code.to_le_bytes())?;
            writer.write_all(&range.start.to_le_bytes())?;
            writer.write_all(&range.length.to_le_bytes())?;
            writer.write_all(&range.public_space_index.to_le_bytes())?;
            writer.write_all(&range.locality_index.to_le_bytes())?;
            writer.write_all(&[range.step])?;
        }

        // Write municipality string table
        offset = 0;
        writer.write_all(&offset.to_le_bytes())?;
        for name in &self.municipalities {
            offset = offset.saturating_add(name.len() as u32);
            writer.write_all(&offset.to_le_bytes())?;
        }
        for name in &self.municipalities {
            writer.write_all(name.as_bytes())?;
        }

        // Write province string table
        offset = 0;
        writer.write_all(&offset.to_le_bytes())?;
        for name in &self.provinces {
            offset = offset.saturating_add(name.len() as u32);
            writer.write_all(&offset.to_le_bytes())?;
        }
        for name in &self.provinces {
            writer.write_all(name.as_bytes())?;
        }

        // Write locality -> municipality index map
        for &m_idx in &self.locality_municipality {
            writer.write_all(&m_idx.to_le_bytes())?;
        }

        // Write municipality -> province index map
        for &p_idx in &self.municipality_province {
            writer.write_all(&[p_idx])?;
        }

        // Write municipality codes
        for &code in &self.municipality_codes {
            writer.write_all(&code.to_le_bytes())?;
        }

        // Write locality codes (BAG woonplaatsidentificatiecode per locality_index)
        for &code in &self.locality_codes {
            writer.write_all(&code.to_le_bytes())?;
        }

        // Write had_suffix flags as one byte each (0 or 1).
        for &flag in &self.locality_had_suffix {
            writer.write_all(&[flag as u8])?;
        }
        for &flag in &self.municipality_had_suffix {
            writer.write_all(&[flag as u8])?;
        }

        Ok(())
    }
}
