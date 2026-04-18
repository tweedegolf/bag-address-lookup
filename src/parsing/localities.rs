// Parses Woonplaats (locality/city/town) objects from the BAG extract.
// BAG catalog §7.2: https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag/catalogus-bag
//
// A Woonplaats is a formally designated area within a municipality. Only
// voorkomens that are part of the active lifecycle on the extract's
// standtechnische datum are kept: not superseded, not retracted, not inactive,
// not flagged NIET BAG, and with a beginGeldigheid that has already passed.

use std::{collections::HashMap, io::BufRead};

use quick_xml::{events::Event, reader::Reader};

use super::{
    municipalities::strip_province_suffix,
    xml_utils::{
        BEGIN_VALIDITY_TAG, END_VALIDITY_TAG, TIJDSTIP_INACTIEF_TAG, TIJDSTIP_NIETBAG_TAG,
        VOORKOMEN_ID_TAG, VoorkomenState, read_simple_tag,
    },
};

const WP_TAG: &[u8] = b"Objecten:Woonplaats";
// §7.2.1 identificatie - unique four-digit national identifier
const ID_TAG: &[u8] = b"Objecten:identificatie";
// §7.2.2 naam - official locality name
const NAME_TAG: &[u8] = b"Objecten:naam";
// §7.2.3 status - "Woonplaats aangewezen" (active) or "Woonplaats ingetrokken" (retracted)
const STATUS_TAG: &[u8] = b"Objecten:status";
const STATUS_RETRACTED: &str = "Woonplaats ingetrokken";

#[derive(Debug, PartialEq, Eq)]
pub struct Locality {
    pub id: u16,
    pub name: String,
    /// True when a province suffix was stripped from the BAG name (e.g.
    /// `Hengelo (Gld)` → `Hengelo`). Treated as not-unique regardless of
    /// current duplicates.
    pub had_suffix: bool,
}

/// Parse BAG locality XML data into structured locality records.
///
/// `reference_date` is the extract's standtechnische datum (YYYY-MM-DD);
/// voorkomens with a future `beginGeldigheid` are excluded.
pub fn parse_localities<R: BufRead>(
    reader: R,
    reference_date: &str,
) -> Result<Vec<Locality>, quick_xml::Error> {
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    // Dedup by identificatiecode, keeping the voorkomen with the highest
    // voorkomenidentificatie (the latest materially-valid version).
    let mut by_id: HashMap<u16, (u32, Locality)> = HashMap::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == WP_TAG => {
                if let Some((voorkomen_id, locality)) =
                    parse_woonplaats(&mut reader, &mut buf, reference_date)?
                {
                    by_id
                        .entry(locality.id)
                        .and_modify(|slot| {
                            if voorkomen_id > slot.0 {
                                *slot = (voorkomen_id, Locality {
                                    id: locality.id,
                                    name: locality.name.clone(),
                                    had_suffix: locality.had_suffix,
                                });
                            }
                        })
                        .or_insert((voorkomen_id, locality));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let mut out: Vec<Locality> = by_id.into_values().map(|(_, loc)| loc).collect();
    out.sort_by_key(|l| l.id);
    Ok(out)
}

fn parse_woonplaats<B: BufRead>(
    reader: &mut Reader<B>,
    buf: &mut Vec<u8>,
    reference_date: &str,
) -> Result<Option<(u32, Locality)>, quick_xml::Error> {
    let mut id = None;
    let mut name = None;
    let mut retracted = false;
    let mut state = VoorkomenState::default();

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) if e.name().as_ref() == ID_TAG => {
                if let Some(value) = read_simple_tag(reader, ID_TAG, buf)? {
                    id = Some(value.parse().expect("Failed to parse locality id"));
                }
            }
            Event::Start(e) if e.name().as_ref() == NAME_TAG => {
                if let Some(value) = read_simple_tag(reader, NAME_TAG, buf)? {
                    name = Some(value);
                }
            }
            Event::Start(e) if e.name().as_ref() == STATUS_TAG => {
                if let Some(value) = read_simple_tag(reader, STATUS_TAG, buf)?
                    && value == STATUS_RETRACTED
                {
                    retracted = true;
                }
            }
            Event::Start(e) if e.name().as_ref() == END_VALIDITY_TAG => {
                state.eind_geldigheid = true;
                let _ = read_simple_tag(reader, END_VALIDITY_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == BEGIN_VALIDITY_TAG => {
                state.begin_geldigheid = read_simple_tag(reader, BEGIN_VALIDITY_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == TIJDSTIP_INACTIEF_TAG => {
                state.tijdstip_inactief = true;
                let _ = read_simple_tag(reader, TIJDSTIP_INACTIEF_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == TIJDSTIP_NIETBAG_TAG => {
                state.tijdstip_nietbag = true;
                let _ = read_simple_tag(reader, TIJDSTIP_NIETBAG_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == VOORKOMEN_ID_TAG => {
                if let Some(value) = read_simple_tag(reader, VOORKOMEN_ID_TAG, buf)? {
                    state.voorkomen_id = value.parse().ok();
                }
            }
            Event::End(e) if e.name().as_ref() == WP_TAG => break,
            Event::Eof => break,
            _ => {}
        }
    }

    if retracted || state.is_inactive(reference_date) {
        return Ok(None);
    }

    match (id, name) {
        (Some(id), Some(mut name)) => {
            let stripped = strip_province_suffix(&name);
            let had_suffix = stripped.len() != name.len();
            if had_suffix {
                let new_name = stripped.to_string();
                eprintln!(
                    "Warning: Stripped province suffix from locality '{name}' -> '{new_name}'"
                );
                name = new_name;
            }
            Ok(Some((
                state.voorkomen_id.unwrap_or(0),
                Locality {
                    id,
                    name,
                    had_suffix,
                },
            )))
        }
        _ => Ok(None),
    }
}
