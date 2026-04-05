// Downloads and parses the CBS (Centraal Bureau voor de Statistiek) municipality
// data, mapping each gemeente code to its name and province.
//
// The CBS "Gebieden in Nederland" table is published annually with a new table ID.
// We auto-detect the latest table via the OData catalog, falling back to a known ID.

use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::log_with_elapsed;

static CBS_TABLE_ID_FALLBACK: &str = "86247NED";
static CBS_PATH: &str = "data/municipalities.json";

#[derive(Debug)]
pub struct Municipality {
    pub code: u16,
    pub name: String,
    pub province: String,
}

/// Download (if needed) and parse CBS municipality data.
pub fn load_municipalities(start: Instant) -> Result<Vec<Municipality>, Box<dyn Error>> {
    let path = ensure_cbs_available(start)?;
    let municipalities = parse_cbs_json(&path)?;
    log_with_elapsed(
        start,
        &format!(
            "Parsed {} municipalities from CBS data",
            municipalities.len()
        ),
    );
    Ok(municipalities)
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

fn ensure_cbs_available(start: Instant) -> Result<PathBuf, Box<dyn Error>> {
    let path = PathBuf::from(CBS_PATH);
    if path.exists() {
        log_with_elapsed(start, "Using existing CBS municipalities file.");
        return Ok(path);
    }

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

    let status = std::process::Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&path)
        .arg(&url)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to download CBS data from {url}").into());
    }

    log_with_elapsed(start, "CBS download complete.");
    Ok(path)
}

fn parse_cbs_json(path: &Path) -> Result<Vec<Municipality>, Box<dyn Error>> {
    let data = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&data)?;
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
        let name = entry["Naam_2"]
            .as_str()
            .ok_or("CBS JSON: missing Naam_2")?
            .trim()
            .to_string();
        let province = entry["Naam_29"]
            .as_str()
            .ok_or("CBS JSON: missing Naam_29")?
            .trim()
            .to_string();

        municipalities.push(Municipality {
            code,
            name,
            province,
        });
    }

    Ok(municipalities)
}
