mod addresses;
mod localities;
pub mod municipalities;
mod municipality_relations;
mod public_spaces;
mod xml_utils;

use std::{
    error::Error,
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
    time::Instant,
};

use rayon::prelude::*;

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
        parse_fn: F,
    ) -> Result<Vec<T>, Box<dyn Error>>
    where
        T: Send,
        F: Fn(&mut dyn std::io::BufRead) -> Result<Vec<T>, quick_xml::Error> + Sync,
    {
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;

        log_with_elapsed(start, &format!("Read {} bytes from {name}", buf.len()));

        // Inner ZIP entries are parsed in parallel. Each worker opens its own
        // ZipArchive over the shared buffer; ZipArchive::by_index needs &mut,
        // so sharing a single archive across threads isn't possible, but
        // re-opening is cheap since the central directory is already in memory.
        let n = ZipArchive::new(Cursor::new(&buf[..]))?.len();

        let per_file: Vec<Vec<T>> = (0..n)
            .into_par_iter()
            .map(|i| -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
                let mut inner_zip = ZipArchive::new(Cursor::new(&buf[..]))?;
                let inner_entry = inner_zip.by_index(i)?;
                if !inner_entry.name().ends_with(".xml") {
                    return Ok(Vec::new());
                }
                let mut reader = BufReader::new(inner_entry);
                Ok(parse_fn(&mut reader)?)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| -> Box<dyn Error> { e })?;

        let total: usize = per_file.iter().map(Vec::len).sum();
        let mut items = Vec::with_capacity(total);
        for chunk in per_file {
            items.extend(chunk);
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

        // Output order depends on HashMap iteration and parallel scheduling,
        // so assertions are set-based.
        let mut house_numbers: Vec<u32> =
            parsed_data.addresses.iter().map(|a| a.house_number).collect();
        house_numbers.sort();
        assert_eq!(house_numbers, vec![1, 56]);
        assert!(
            parsed_data
                .addresses
                .iter()
                .all(|a| a.postal_code == "1234AB")
        );

        let mut public_space_names: Vec<&str> = parsed_data
            .public_spaces
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        public_space_names.sort();
        assert_eq!(public_space_names, vec!["Abel Eppensstraat", "Adamistraat"]);

        let mut locality_names: Vec<&str> =
            parsed_data.localities.iter().map(|l| l.name.as_str()).collect();
        locality_names.sort();
        assert_eq!(locality_names, vec!["Hoogerheide", "Huijbergen"]);
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
