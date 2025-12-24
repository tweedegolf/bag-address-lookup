mod addresses;
mod localities;
mod public_spaces;
mod xml_utils;

use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    time::Instant,
};

pub use addresses::{Address, parse_addresses};
pub use localities::{Locality, parse_localities};
pub use public_spaces::{PublicSpace, parse_public_spaces};
use zip::ZipArchive;

use crate::log_with_elapsed;

#[derive(Default, Debug)]
pub struct ParsedData {
    pub addresses: Vec<addresses::Address>,
    pub public_spaces: Vec<public_spaces::PublicSpace>,
    pub localities: Vec<localities::Locality>,
}

impl ParsedData {
    /// Load and parse BAG data from a zip archive into structured records.
    pub fn from_bag_zip(zip_path: &Path, start: Instant) -> Result<ParsedData, Box<dyn Error>> {
        let f = File::open(zip_path)?;
        let mut zip = ZipArchive::new(f)?;
        let mut data = ParsedData::default();

        for index in 0..zip.len() {
            let mut entry = zip.by_index(index)?;
            let name = entry.name().to_string();

            if entry.is_dir() || !name.ends_with(".zip") {
                continue;
            }

            match &name[..7] {
                "9999WPL" => {
                    data.localities = ParsedData::parse_nested_xml_zip(
                        start,
                        &mut entry,
                        "localities",
                        |reader| parse_localities(reader),
                    )?;
                }
                "9999OPR" => {
                    data.public_spaces = ParsedData::parse_nested_xml_zip(
                        start,
                        &mut entry,
                        "public spaces",
                        |reader| parse_public_spaces(reader),
                    )?;
                }
                "9999NUM" => {
                    data.addresses = ParsedData::parse_nested_xml_zip(
                        start,
                        &mut entry,
                        "addresses",
                        |reader| parse_addresses(reader),
                    )?;
                }
                _ => {
                    // ignore other files
                }
            }
        }

        Ok(data)
    }

    fn parse_nested_xml_zip<T, F>(
        start: Instant,
        entry: &mut zip::read::ZipFile<'_, File>,
        label: &str,
        mut parse_fn: F,
    ) -> Result<Vec<T>, Box<dyn Error>>
    where
        F: FnMut(&mut dyn std::io::BufRead) -> Result<Vec<T>, quick_xml::Error>,
    {
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;

        log_with_elapsed(start, &format!("Read {} bytes from {name}", buf.len()));
        let cursor = std::io::Cursor::new(buf);
        let mut inner_zip = ZipArchive::new(cursor)?;

        let mut items = Vec::new();

        for i in 0..inner_zip.len() {
            let inner_entry = inner_zip.by_index(i)?;
            let inner_name = inner_entry.name().to_string();

            if !inner_name.ends_with(".xml") {
                continue;
            }

            let mut reader = BufReader::new(inner_entry);
            items.extend(parse_fn(&mut reader)?);

            if i % 100 == 0 {
                log_with_elapsed(
                    start,
                    &format!("Loaded {} files and {} items total.", i + 1, items.len()),
                );
            }
        }

        log_with_elapsed(start, &format!("Parsed {} {label}", items.len()));

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_bag_zip() {
        let test_zip_path = PathBuf::from("test/bag.zip");
        let start = Instant::now();

        let parsed_data = ParsedData::from_bag_zip(&test_zip_path, start).unwrap();

        assert_eq!(parsed_data.addresses[0].house_number, 56);
        assert_eq!(parsed_data.addresses[0].postal_code, "1234AB");
        assert_eq!(parsed_data.addresses[1].house_number, 1);
        assert_eq!(parsed_data.addresses[1].postal_code, "1234AB");

        assert_eq!(parsed_data.public_spaces[0].name, "Abel Eppensstraat");
        assert_eq!(parsed_data.public_spaces[1].name, "Adamistraat");

        assert_eq!(parsed_data.localities[0].name, "Hoogerheide");
        assert_eq!(parsed_data.localities[1].name, "Huijbergen");
    }
}
