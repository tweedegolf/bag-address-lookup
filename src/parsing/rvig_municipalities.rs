// Downloads and parses the RVIG "Landelijke Tabel 33 — Gemeenten" CSV.
//
// RVIG publishes Tabel 33 as part of the BRP landelijke tabellen. Compared to
// the CBS "Gebieden in Nederland" table (our primary source), Tabel 33:
//   - Lists historical/dissolved gemeenten alongside current ones, with a
//     "nieuwe gemeentecode" pointing to the successor and an end date,
//   - Contains non-gemeente placeholder codes (Onbekend, Buitenland, RNI),
//   - Only carries province information as a disambiguating suffix on the
//     name when two gemeenten share a name (e.g. "Bergen (NH)").
//
// We load it as a secondary source to cross-check CBS at build time. Only
// current entries are kept and province suffixes are stripped so names line
// up with the CBS form.
//
// See https://publicaties.rvig.nl/Landelijke_tabellen and LO-451.

use std::{collections::HashMap, error::Error, time::Instant};

use crate::{
    log_with_elapsed,
    parsing::municipalities::{Municipality, strip_province_suffix},
};

static RVIG_URL: &str = "https://publicaties.rvig.nl/media/13307/download";
static RVIG_FALLBACK_PATH: &str = "fallback/rvig_municipalities.csv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RvigMunicipality {
    pub code: u16,
    pub name: String,
    /// True when the RVIG name carried a disambiguating province suffix that
    /// was stripped (e.g. `Bergen (NH)`).
    pub had_suffix: bool,
}

/// Load RVIG Tabel 33, comparing the live source against the committed
/// fallback. Returns the live data when reachable; falls back to the committed
/// copy when RVIG is down. Errors if the live response parses successfully but
/// differs from the fallback — that signals an upstream change the maintainer
/// should review and recommit.
pub fn load_rvig_municipalities(start: Instant) -> Result<Vec<RvigMunicipality>, Box<dyn Error>> {
    let fallback_bytes = std::fs::read(RVIG_FALLBACK_PATH)
        .map_err(|e| format!("Could not read RVIG fallback at {RVIG_FALLBACK_PATH}: {e}"))?;
    let fallback = parse_rvig_csv_bytes(&fallback_bytes)?;

    match fetch_rvig_live(start) {
        Ok(bytes) => match parse_rvig_csv_bytes(&bytes) {
            Ok(live) => {
                if !rvig_match(&live, &fallback) {
                    return Err(format!(
                        "Live RVIG Tabel 33 differs from committed fallback at {RVIG_FALLBACK_PATH}. \
                         Inspect the diff and update the fallback file before continuing."
                    )
                    .into());
                }
                log_with_elapsed(
                    start,
                    &format!(
                        "Parsed {} current municipalities from RVIG Tabel 33 (matches committed fallback)",
                        live.len()
                    ),
                );
                Ok(live)
            }
            Err(e) => {
                log_with_elapsed(
                    start,
                    &format!(
                        "RVIG returned an unparseable response ({e}); using committed fallback at {RVIG_FALLBACK_PATH}"
                    ),
                );
                Ok(fallback)
            }
        },
        Err(e) => {
            log_with_elapsed(
                start,
                &format!(
                    "RVIG unreachable ({e}); using committed fallback at {RVIG_FALLBACK_PATH}"
                ),
            );
            Ok(fallback)
        }
    }
}

fn fetch_rvig_live(start: Instant) -> Result<Vec<u8>, Box<dyn Error>> {
    log_with_elapsed(start, "Downloading RVIG Tabel 33...");

    let output = std::process::Command::new("curl")
        .arg("-sLf")
        .arg(RVIG_URL)
        .output()?;

    if !output.status.success() {
        return Err(format!("Failed to download RVIG Tabel 33 from {RVIG_URL}").into());
    }

    log_with_elapsed(start, "RVIG download complete.");
    Ok(output.stdout)
}

fn rvig_match(a: &[RvigMunicipality], b: &[RvigMunicipality]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut a_sorted = a.to_vec();
    let mut b_sorted = b.to_vec();
    a_sorted.sort_by_key(|m| m.code);
    b_sorted.sort_by_key(|m| m.code);
    a_sorted == b_sorted
}

fn parse_rvig_csv_bytes(bytes: &[u8]) -> Result<Vec<RvigMunicipality>, Box<dyn Error>> {
    let text = decode_utf16_le(bytes)?;
    parse_rvig_csv_text(&text)
}

/// Decode UTF-16 LE bytes (with optional BOM) to a String. RVIG ships the CSV
/// as UTF-16 LE, unlike most Dutch government datasets.
fn decode_utf16_le(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let body = match bytes {
        [0xFF, 0xFE, rest @ ..] => rest,
        _ => bytes,
    };
    if !body.len().is_multiple_of(2) {
        return Err("RVIG CSV: odd byte count in UTF-16 LE payload".into());
    }
    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok(String::from_utf16_lossy(&units))
}

fn parse_rvig_csv_text(text: &str) -> Result<Vec<RvigMunicipality>, Box<dyn Error>> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_matches(|c: char| c == '\u{FEFF}' || c.is_whitespace());
        if trimmed.is_empty() || index == 0 {
            continue;
        }
        let fields = parse_csv_line(trimmed);
        if fields.len() < 5 {
            return Err(format!(
                "RVIG CSV: expected 5 fields on line {}, got {}",
                index + 1,
                fields.len()
            )
            .into());
        }
        let code_str = fields[0].trim();
        let name = fields[1].trim();
        let end_date = fields[4].trim();

        // Keep only current gemeenten (those without an end date).
        if !end_date.is_empty() {
            continue;
        }

        if is_non_gemeente(code_str, name) {
            continue;
        }

        let code: u16 = code_str.parse().map_err(|e| {
            format!(
                "RVIG CSV: invalid code '{code_str}' on line {}: {e}",
                index + 1
            )
        })?;
        let stripped = strip_province_suffix(name);
        let had_suffix = stripped != name;
        if had_suffix {
            eprintln!("Warning: Stripped province suffix from RVIG '{name}' -> '{stripped}'");
        }
        out.push(RvigMunicipality {
            code,
            name: stripped.to_string(),
            had_suffix,
        });
    }
    Ok(out)
}

/// Placeholder codes reserved for non-geographic categories in the BRP.
/// `0000` Onbekend, `0997`..`0999` Niet-GBA registrations, `1999` RNI.
fn is_non_gemeente(code: &str, name: &str) -> bool {
    matches!(code, "0000" | "0997" | "0998" | "0999" | "1999") || name.contains("(Niet GBA)")
}

/// Minimal CSV line parser: double-quoted fields, `""` escapes a literal
/// quote, comma separator outside quotes. RVIG Tabel 33 follows this shape.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, in_quotes) {
            ('"', true) if chars.peek() == Some(&'"') => {
                chars.next();
                current.push('"');
            }
            ('"', true) => in_quotes = false,
            ('"', false) => in_quotes = true,
            (',', false) => out.push(std::mem::take(&mut current)),
            (ch, _) => current.push(ch),
        }
    }
    out.push(current);
    out
}

/// Log important differences between CBS (primary) and RVIG (secondary) so
/// any drift is visible when (re)building the database.
pub fn report_differences_vs_cbs(rvig: &[RvigMunicipality], cbs: &[Municipality], start: Instant) {
    let cbs_by_code: HashMap<u16, &Municipality> = cbs.iter().map(|m| (m.code, m)).collect();
    let rvig_by_code: HashMap<u16, &RvigMunicipality> = rvig.iter().map(|m| (m.code, m)).collect();

    let mut only_in_cbs: Vec<&Municipality> = cbs
        .iter()
        .filter(|m| !rvig_by_code.contains_key(&m.code))
        .collect();
    only_in_cbs.sort_by_key(|m| m.code);

    let mut only_in_rvig: Vec<&RvigMunicipality> = rvig
        .iter()
        .filter(|m| !cbs_by_code.contains_key(&m.code))
        .collect();
    only_in_rvig.sort_by_key(|m| m.code);

    let mut name_mismatches: Vec<(u16, &str, &str)> = rvig
        .iter()
        .filter_map(|r| {
            cbs_by_code
                .get(&r.code)
                .filter(|c| c.name != r.name)
                .map(|c| (r.code, c.name.as_str(), r.name.as_str()))
        })
        .collect();
    name_mismatches.sort_by_key(|&(code, _, _)| code);

    log_with_elapsed(
        start,
        &format!(
            "CBS vs RVIG Tabel 33: {} CBS-only, {} RVIG-only, {} name mismatches",
            only_in_cbs.len(),
            only_in_rvig.len(),
            name_mismatches.len(),
        ),
    );
    for m in only_in_cbs {
        eprintln!(
            "  CBS-only gemeente {:04}: {} ({})",
            m.code, m.name, m.province
        );
    }
    for m in only_in_rvig {
        eprintln!("  RVIG-only gemeente {:04}: {}", m.code, m.name);
    }
    for (code, cbs_name, rvig_name) in name_mismatches {
        eprintln!("  Name differs for gemeente {code:04}: CBS '{cbs_name}' vs RVIG '{rvig_name}'");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_utf16_le_handles_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for c in "abc".chars() {
            bytes.push(c as u8);
            bytes.push(0);
        }
        assert_eq!(decode_utf16_le(&bytes).unwrap(), "abc");
    }

    #[test]
    fn parse_csv_line_handles_quoted_fields() {
        let fields = parse_csv_line(r#""0014","Groningen","","",""#);
        assert_eq!(fields, vec!["0014", "Groningen", "", "", ""]);
    }

    #[test]
    fn parse_csv_line_unescapes_doubled_quotes() {
        let fields = parse_csv_line(r#""a","b""c","d""#);
        assert_eq!(fields, vec!["a", r#"b"c"#, "d"]);
    }

    #[test]
    fn parser_skips_header_historical_and_non_gemeente() {
        let csv = "\"92.10 Gemeentecode\",\"92.11 Gemeentenaam\",\"92.12 Nieuwe gemeentecode\",\"99.98 Datum ingang tabelregel\",\"99.99 Datum beëindiging tabelregel\"\n\
                   \"0000\",\"Onbekend\",\"\",\"\",\"\"\n\
                   \"0001\",\"Adorp\",\"0053\",\"\",\"19900101\"\n\
                   \"0014\",\"Groningen\",\"\",\"\",\"\"\n\
                   \"0373\",\"Bergen (NH)\",\"\",\"\",\"\"\n\
                   \"0998\",\"Buitenland (Niet GBA)\",\"\",\"\",\"19901001\"\n\
                   \"1999\",\"Registratie Niet Ingezetenen (RNI)\",\"\",\"20140106\",\"\"\n";

        let parsed = parse_rvig_csv_text(csv).unwrap();

        assert_eq!(
            parsed,
            vec![
                RvigMunicipality {
                    code: 14,
                    name: "Groningen".to_string(),
                    had_suffix: false,
                },
                RvigMunicipality {
                    code: 373,
                    name: "Bergen".to_string(),
                    had_suffix: true,
                },
            ]
        );
    }

    #[test]
    fn report_differences_flags_name_and_membership_diffs() {
        let cbs = vec![
            Municipality {
                code: 14,
                name: "Groningen".to_string(),
                province: "GR".to_string(),
                had_suffix: false,
            },
            Municipality {
                code: 518,
                name: "'s-Gravenhage".to_string(),
                province: "ZH".to_string(),
                had_suffix: false,
            },
        ];
        let rvig = vec![
            RvigMunicipality {
                code: 14,
                name: "Groningen".to_string(),
                had_suffix: false,
            },
            RvigMunicipality {
                code: 518,
                name: "Den Haag".to_string(),
                had_suffix: false,
            },
            RvigMunicipality {
                code: 1992,
                name: "Voorne aan Zee".to_string(),
                had_suffix: false,
            },
        ];

        // Smoke test only: the function logs to stderr but has no return value.
        report_differences_vs_cbs(&rvig, &cbs, Instant::now());
    }
}
