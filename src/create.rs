use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{Database, log_with_elapsed, parsing::ParsedData};

static DOWNLOAD_URL: &str =
    "https://service.pdok.nl/kadaster/adressen/atom/v1_0/downloads/lvbag-extract-nl.zip";
static ZIP_PATH: &str = "data/bag.zip";
static OUTPUT_PATH: &str = "data/bag.bin";

/// Build the BAG database file if it does not already exist.
pub fn create_database() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let output_path = Path::new(OUTPUT_PATH);

    if output_path.exists() && output_path.metadata()?.len() > 0 {
        log_with_elapsed(start, "BAG database already exists, skipping creation.");
        return Ok(());
    }

    let zip_path = ensure_zip_available(start)?;
    let data = ParsedData::from_bag_zip(&zip_path, start)?;
    let database = Database::from_parsed_data(data)?;

    log_with_elapsed(
        start,
        &format!(
            "Created database structure: {} localities, {} public spaces, {} address ranges.",
            database.localities.len(),
            database.public_spaces.len(),
            database.ranges.len()
        ),
    );

    database.encode(output_path)?;

    log_with_elapsed(start, &format!("Encoded database written to {OUTPUT_PATH}"));

    Ok(())
}

fn ensure_zip_available(start: Instant) -> Result<PathBuf, Box<dyn Error>> {
    let zip_path = PathBuf::from(ZIP_PATH);

    if zip_path.exists() {
        log_with_elapsed(start, "Using existing BAG zip file.");
        return Ok(zip_path);
    }

    log_with_elapsed(start, "Downloading BAG data...");

    let status = std::process::Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&zip_path)
        .arg(DOWNLOAD_URL)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to download file from {DOWNLOAD_URL}").into());
    }

    log_with_elapsed(start, "Download complete.");

    Ok(zip_path)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::Instant};

    use crate::{Database, parsing::ParsedData};

    #[test]
    fn test_create_database() {
        let start = Instant::now();
        let zip_path = PathBuf::from("test/bag.zip");

        #[cfg(feature = "compressed_database")]
        let output_path = PathBuf::from("test/bag.bin");

        #[cfg(not(feature = "compressed_database"))]
        let output_path = PathBuf::from("test/bag_uncompressed.bin");

        let data = ParsedData::from_bag_zip(&zip_path, start).unwrap();
        let database = Database::from_parsed_data(data).unwrap();

        // get filesize of exsisting database file
        let old_metadata = std::fs::metadata(&output_path).unwrap();

        database.encode(&output_path).unwrap();

        let metadata = std::fs::metadata(&output_path).unwrap();
        let modified_time = metadata.modified().unwrap();
        let previous_modified_time = old_metadata.modified().unwrap();

        assert_eq!(metadata.len(), old_metadata.len());
        assert!(modified_time >= previous_modified_time);
    }
}
