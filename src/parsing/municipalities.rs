// Downloads and parses the CBS (Centraal Bureau voor de Statistiek) municipality
// data, mapping each gemeente code to its name and province.
//
// The CBS "Gebieden in Nederland" table is published annually with a new table ID.
// We auto-detect the latest table via the OData catalog, falling back to a known ID.

use std::{error::Error, time::Instant};

use crate::log_with_elapsed;

static CBS_TABLE_ID_FALLBACK: &str = "86247NED";
static CBS_FALLBACK_PATH: &str = "fallback/municipalities.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Municipality {
    pub code: u16,
    pub name: String,
    /// Two-letter province code (e.g. "NH", "ZH", "UT").
    pub province: String,
    /// True when the CBS name carried a disambiguating province suffix that
    /// was stripped (e.g. `Hengelo (O.)`). CBS retains such suffixes because
    /// the name has historically been ambiguous; we treat those entries as
    /// not unique regardless of whether duplicates still exist today.
    pub had_suffix: bool,
}

/// Remove the disambiguating province suffix appended to shared names.
/// Handles three forms:
/// - CBS form with parens and a trailing dot ("Hengelo (O.)", "Bergen (NH.)"),
/// - BAG municipality form with parens, no dot ("Hengelo (Gld)"),
/// - BAG locality form without parens ("Bergen L", "Driehuis NH", "Beuningen Gld").
///
/// The unparenthesized form is matched against a fixed whitelist of Dutch
/// province abbreviations so names like "De Bilt", "Ten Boer", "Bergen op Zoom"
/// are not mistakenly stripped.
pub(crate) fn strip_province_suffix(name: &str) -> &str {
    if let Some(stripped) = strip_parenthesized_suffix(name) {
        return stripped;
    }
    if let Some(stripped) = strip_unparenthesized_suffix(name) {
        return stripped;
    }
    name
}

fn strip_parenthesized_suffix(name: &str) -> Option<&str> {
    let open = name.rfind(" (")?;
    let inner_end = name.strip_suffix(')')?;
    let inside = inner_end[open + 2..].trim_end_matches('.');
    if !inside.is_empty() && inside.len() <= 3 && inside.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(name[..open].trim_end());
    }
    None
}

fn strip_unparenthesized_suffix(name: &str) -> Option<&str> {
    let space = name.rfind(' ')?;
    let suffix = name[space + 1..].trim_end_matches('.');
    let prefix = name[..space].trim_end();
    if prefix.is_empty() {
        return None;
    }
    is_province_abbreviation(suffix).then_some(prefix)
}

/// Recognised Dutch province abbreviations as observed in BAG/CBS/RVIG name
/// suffixes. Matched case-insensitively.
fn is_province_abbreviation(token: &str) -> bool {
    matches!(
        token.to_ascii_uppercase().as_str(),
        "L" | "LB"      // Limburg
        | "NH"           // Noord-Holland
        | "ZH"           // Zuid-Holland
        | "NB"           // Noord-Brabant
        | "GLD"          // Gelderland
        | "OV"           // Overijssel
        | "UT"           // Utrecht
        | "GN"           // Groningen
        | "DR"           // Drenthe
        | "FR"           // Friesland
        | "FL"           // Flevoland
        | "ZE" // Zeeland
    )
}

/// Map a CBS province name to its two-letter code.
fn province_code(name: &str) -> String {
    match name {
        "Drenthe" => "DR",
        "Flevoland" => "FL",
        "Fryslân" | "Friesland" => "FR",
        "Gelderland" => "GE",
        "Groningen" => "GR",
        "Limburg" => "LI",
        "Noord-Brabant" => "NB",
        "Noord-Holland" => "NH",
        "Overijssel" => "OV",
        "Utrecht" => "UT",
        "Zeeland" => "ZE",
        "Zuid-Holland" => "ZH",
        _ => return name.to_string(),
    }
    .to_string()
}

/// Load CBS municipality data, comparing the live source against the committed
/// fallback. Returns the live data when reachable; falls back to the committed
/// copy when CBS is down. Errors if the live response parses successfully but
/// differs from the fallback — that signals an upstream change the maintainer
/// should review and recommit.
pub fn load_municipalities(start: Instant) -> Result<Vec<Municipality>, Box<dyn Error>> {
    let fallback_text = std::fs::read_to_string(CBS_FALLBACK_PATH)
        .map_err(|e| format!("Could not read CBS fallback at {CBS_FALLBACK_PATH}: {e}"))?;
    let fallback = parse_cbs_json_text(&fallback_text)?;

    match fetch_cbs_live(start) {
        Ok(bytes) => match parse_cbs_json_text(&String::from_utf8_lossy(&bytes)) {
            Ok(live) => {
                if !municipalities_match(&live, &fallback) {
                    return Err(format!(
                        "Live CBS data differs from committed fallback at {CBS_FALLBACK_PATH}. \
                         Inspect the diff and update the fallback file before continuing."
                    )
                    .into());
                }
                log_with_elapsed(
                    start,
                    &format!(
                        "Parsed {} municipalities from CBS (matches committed fallback)",
                        live.len()
                    ),
                );
                Ok(live)
            }
            Err(e) => {
                log_with_elapsed(
                    start,
                    &format!(
                        "CBS returned an unparseable response ({e}); using committed fallback at {CBS_FALLBACK_PATH}"
                    ),
                );
                Ok(fallback)
            }
        },
        Err(e) => {
            log_with_elapsed(
                start,
                &format!("CBS unreachable ({e}); using committed fallback at {CBS_FALLBACK_PATH}"),
            );
            Ok(fallback)
        }
    }
}

/// Compare two municipality lists irrespective of source ordering.
fn municipalities_match(a: &[Municipality], b: &[Municipality]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut a_sorted = a.to_vec();
    let mut b_sorted = b.to_vec();
    a_sorted.sort_by_key(|m| m.code);
    b_sorted.sort_by_key(|m| m.code);
    a_sorted == b_sorted
}

/// Query the CBS OData catalog to find the latest "Gebieden in Nederland" table ID.
fn detect_latest_table_id(start: Instant) -> Result<String, Box<dyn Error>> {
    let catalog_url = "https://opendata.cbs.nl/ODataCatalog/Tables?\
        $filter=substringof(%27Gebieden%20in%20Nederland%27,%20Title)\
        &$select=Identifier&$format=json&$orderby=Title%20desc&$top=1";

    let output = std::process::Command::new("curl")
        .arg("-sL")
        .arg(catalog_url)
        .output()?;

    if !output.status.success() {
        return Err("CBS catalog request failed".into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let id = json["value"][0]["Identifier"]
        .as_str()
        .ok_or("CBS catalog: missing Identifier")?
        .trim()
        .to_string();

    log_with_elapsed(start, &format!("Detected latest CBS table: {id}"));
    Ok(id)
}

fn fetch_cbs_live(start: Instant) -> Result<Vec<u8>, Box<dyn Error>> {
    let table_id = detect_latest_table_id(start).unwrap_or_else(|e| {
        log_with_elapsed(
            start,
            &format!(
                "Could not detect latest CBS table ({e}), falling back to {CBS_TABLE_ID_FALLBACK}"
            ),
        );
        CBS_TABLE_ID_FALLBACK.to_string()
    });

    let url = format!(
        "https://opendata.cbs.nl/ODataFeed/odata/{table_id}/TypedDataSet\
         ?$select=Code_1,Naam_2,Code_28,Naam_29&$format=json"
    );

    log_with_elapsed(start, "Downloading CBS municipality data...");

    let output = std::process::Command::new("curl")
        .arg("-sLf")
        .arg(&url)
        .output()?;

    if !output.status.success() {
        return Err(format!("Failed to download CBS data from {url}").into());
    }

    log_with_elapsed(start, "CBS download complete.");
    Ok(output.stdout)
}

fn parse_cbs_json_text(data: &str) -> Result<Vec<Municipality>, Box<dyn Error>> {
    let json: serde_json::Value = serde_json::from_str(data)?;
    let entries = json["value"]
        .as_array()
        .ok_or("CBS JSON: missing 'value' array")?;

    let mut municipalities = Vec::with_capacity(entries.len());
    for entry in entries {
        let code_str = entry["Code_1"]
            .as_str()
            .ok_or("CBS JSON: missing Code_1")?
            .trim();
        let code: u16 = code_str
            .strip_prefix("GM")
            .ok_or_else(|| format!("CBS JSON: expected GM prefix in '{code_str}'"))?
            .parse()?;
        let raw_name = entry["Naam_2"]
            .as_str()
            .ok_or("CBS JSON: missing Naam_2")?
            .trim();
        let stripped = strip_province_suffix(raw_name);
        let had_suffix = stripped != raw_name;
        if had_suffix {
            eprintln!(
                "Warning: Stripped province suffix from municipality '{raw_name}' -> '{stripped}'"
            );
        }
        let name = stripped.to_string();
        let province_name = entry["Naam_29"]
            .as_str()
            .ok_or("CBS JSON: missing Naam_29")?
            .trim();
        let province = province_code(province_name);

        municipalities.push(Municipality {
            code,
            name,
            province,
            had_suffix,
        });
    }

    Ok(municipalities)
}

#[cfg(test)]
mod tests {
    use super::strip_province_suffix;

    #[test]
    fn strips_cbs_dotted_suffixes() {
        assert_eq!(strip_province_suffix("Hengelo (O.)"), "Hengelo");
        assert_eq!(strip_province_suffix("Bergen (NH.)"), "Bergen");
        assert_eq!(strip_province_suffix("Bergen (L.)"), "Bergen");
    }

    #[test]
    fn strips_bag_dotless_suffixes() {
        assert_eq!(strip_province_suffix("Hengelo (Gld)"), "Hengelo");
        assert_eq!(strip_province_suffix("Bergen (NH)"), "Bergen");
    }

    #[test]
    fn leaves_unrelated_parentheticals_intact() {
        assert_eq!(strip_province_suffix("Foo (123)"), "Foo (123)");
        assert_eq!(strip_province_suffix("Foo (Bar Baz)"), "Foo (Bar Baz)");
        assert_eq!(
            strip_province_suffix("Something (longer)"),
            "Something (longer)"
        );
        assert_eq!(strip_province_suffix("Plain"), "Plain");
        assert_eq!(strip_province_suffix("Foo ()"), "Foo ()");
    }

    #[test]
    fn strips_unparenthesized_province_suffixes() {
        assert_eq!(strip_province_suffix("Bergen L"), "Bergen");
        assert_eq!(strip_province_suffix("Afferden L"), "Afferden");
        assert_eq!(strip_province_suffix("Well L"), "Well");
        assert_eq!(strip_province_suffix("Driehuis NH"), "Driehuis");
        assert_eq!(strip_province_suffix("Oosterend Nh"), "Oosterend");
        assert_eq!(strip_province_suffix("Beers NB"), "Beers");
        assert_eq!(strip_province_suffix("Beuningen Gld"), "Beuningen");
        assert_eq!(strip_province_suffix("Elst Ut"), "Elst");
        assert_eq!(strip_province_suffix("Haren Gn"), "Haren");
        assert_eq!(strip_province_suffix("Harkstede GN"), "Harkstede");
    }

    #[test]
    fn leaves_lookalike_locality_names_intact() {
        // Names that end in a short capitalized word that is not a province
        // abbreviation must be preserved verbatim.
        for name in [
            "De Bilt",
            "De Lier",
            "Den Burg",
            "Ten Boer",
            "Ter Aar",
            "Oud Ade",
            "Smalle Ee",
            "Bergen aan Zee",
            "Bergen op Zoom",
            "Bavel AC",
            "Ulvenhout AC",
        ] {
            assert_eq!(strip_province_suffix(name), name, "should not strip {name}");
        }
    }
}
