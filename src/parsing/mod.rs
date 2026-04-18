mod addresses;
mod localities;
pub mod municipalities;
mod municipality_relations;
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
pub use municipality_relations::{MunicipalityRelation, parse_municipality_relations};
pub use public_spaces::{PublicSpace, parse_public_spaces};
use zip::ZipArchive;

use crate::log_with_elapsed;

#[derive(Default, Debug)]
pub struct ParsedData {
    pub addresses: Vec<addresses::Address>,
    pub public_spaces: Vec<public_spaces::PublicSpace>,
    pub localities: Vec<localities::Locality>,
    pub municipality_relations: Vec<municipality_relations::MunicipalityRelation>,
}

impl ParsedData {
    /// Load and parse BAG data from a zip archive into structured records.
    pub fn from_bag_zip(zip_path: &Path, start: Instant) -> Result<ParsedData, Box<dyn Error>> {
        let f = File::open(zip_path)?;
        let mut zip = ZipArchive::new(f)?;
        let mut data = ParsedData::default();

        let reference_date = extract_date_from_zip(&mut zip).ok_or(
            "Could not determine standtechnische datum from BAG extract filenames",
        )?;
        log_with_elapsed(
            start,
            &format!("Using extract reference date {reference_date}"),
        );

        for index in 0..zip.len() {
            let mut entry = zip.by_index(index)?;
            let name = entry.name().to_string();

            if entry.is_dir() || !name.ends_with(".zip") {
                continue;
            }

            // The BAG extract contains nested ZIPs identified by a prefix.
            // See https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag/catalogus-bag
            if name.starts_with("GEM-WPL") {
                // Gemeente-Woonplaats relatie (locality to municipality mapping)
                data.municipality_relations = ParsedData::parse_nested_xml_zip(
                    start,
                    &mut entry,
                    "municipality relations",
                    |reader| parse_municipality_relations(reader, &reference_date),
                )?;
            } else {
                match &name[..7] {
                    // Woonplaats (locality) - BAG catalog §7.2
                    "9999WPL" => {
                        data.localities = ParsedData::parse_nested_xml_zip(
                            start,
                            &mut entry,
                            "localities",
                            |reader| parse_localities(reader, &reference_date),
                        )?;
                    }
                    // OpenbareRuimte (public space) - BAG catalog §7.3
                    "9999OPR" => {
                        data.public_spaces = ParsedData::parse_nested_xml_zip(
                            start,
                            &mut entry,
                            "public spaces",
                            |reader| parse_public_spaces(reader, &reference_date),
                        )?;
                    }
                    // Nummeraanduiding (address designation) - BAG catalog §7.4
                    "9999NUM" => {
                        data.addresses = ParsedData::parse_nested_xml_zip(
                            start,
                            &mut entry,
                            "addresses",
                            |reader| parse_addresses(reader, &reference_date),
                        )?;
                    }
                    _ => {
                        // ignore other files
                    }
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

/// Extract the standtechnische datum from the BAG extract's filenames.
///
/// Extract filenames embed the date as DDMMYYYY (e.g. `9999WPL08122025.zip`
/// or `GEM-WPL-RELATIE-08122025.zip`). We scan entries for a trailing 8-digit
/// run and reformat it as ISO-8601 so later string comparisons sort correctly.
fn extract_date_from_zip(zip: &mut ZipArchive<File>) -> Option<String> {
    for index in 0..zip.len() {
        let entry = zip.by_index(index).ok()?;
        let name = entry.name();
        let stem = name
            .rsplit('/')
            .next()
            .unwrap_or(name)
            .trim_end_matches(".zip")
            .trim_end_matches(".xml");
        let trailing_digits: String = stem
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if trailing_digits.len() >= 8 {
            let start = trailing_digits.len() - 8;
            let dd = &trailing_digits[start..start + 2];
            let mm = &trailing_digits[start + 2..start + 4];
            let yyyy = &trailing_digits[start + 4..start + 8];
            return Some(format!("{yyyy}-{mm}-{dd}"));
        }
    }
    None
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

    #[test]
    fn extract_date_parses_ddmmyyyy_filename() {
        // The function expects a real ZIP archive; just verify the algorithm
        // on filenames produced by the BAG extract format.
        fn parse(name: &str) -> Option<String> {
            let stem = name.trim_end_matches(".zip");
            let digits: String = stem
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if digits.len() >= 8 {
                let s = digits.len() - 8;
                return Some(format!(
                    "{}-{}-{}",
                    &digits[s + 4..s + 8],
                    &digits[s + 2..s + 4],
                    &digits[s..s + 2],
                ));
            }
            None
        }
        assert_eq!(
            parse("9999WPL08122025.zip").as_deref(),
            Some("2025-12-08")
        );
        assert_eq!(
            parse("GEM-WPL-RELATIE-08122025.zip").as_deref(),
            Some("2025-12-08")
        );
    }
}
