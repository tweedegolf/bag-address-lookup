// Parses OpenbareRuimte (public space / street) objects from the BAG extract.
// BAG catalog §7.3: https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag/catalogus-bag
//
// An OpenbareRuimte is a public space (usually a street) within a Woonplaats.
// Only currently valid records with status "Naamgeving uitgegeven" are included.

use std::{collections::HashMap, io::BufRead};

use quick_xml::{events::Event, reader::Reader};

use super::xml_utils::{
    BEGIN_VALIDITY_TAG, END_VALIDITY_TAG, TIJDSTIP_INACTIEF_TAG, TIJDSTIP_NIETBAG_TAG,
    VOORKOMEN_ID_TAG, VoorkomenState, read_simple_tag,
};

const OPR_TAG: &[u8] = b"Objecten:OpenbareRuimte";
// §7.3.1 identificatie - 16-digit national identifier
const ID_TAG: &[u8] = b"Objecten:identificatie";
// §7.3.2 naam - official public space name (max 80 characters)
const NAME_TAG: &[u8] = b"Objecten:naam";
// §7.3.6 ligtIn - reference to the Woonplaats this public space belongs to
const LOCALITY_REF_TAG: &[u8] = b"Objecten-ref:WoonplaatsRef";
// §7.3.4 status - lifecycle status of the public space
const STATUS_TAG: &[u8] = b"Objecten:status";
// Only include public spaces where a name has been officially issued
const ISSUED_STATUS: &str = "Naamgeving uitgegeven";

#[derive(Debug, PartialEq, Eq)]
pub struct PublicSpace {
    pub id: String,
    pub name: String,
    pub locality_id: u16,
}

/// Parse BAG public space XML data into structured public space records.
///
/// `reference_date` is the extract's standtechnische datum (YYYY-MM-DD);
/// voorkomens with a future `beginGeldigheid` are excluded.
pub fn parse_public_spaces<R: BufRead>(
    source: R,
    reference_date: &str,
) -> Result<Vec<PublicSpace>, quick_xml::Error> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut by_id: HashMap<String, (u32, PublicSpace)> = HashMap::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == OPR_TAG => {
                if let Some((voorkomen_id, public_space)) =
                    parse_openbare_ruimte(&mut reader, &mut buf, reference_date)?
                {
                    let id = public_space.id.clone();
                    by_id
                        .entry(id)
                        .and_modify(|slot| {
                            if voorkomen_id > slot.0 {
                                *slot = (voorkomen_id, PublicSpace {
                                    id: public_space.id.clone(),
                                    name: public_space.name.clone(),
                                    locality_id: public_space.locality_id,
                                });
                            }
                        })
                        .or_insert((voorkomen_id, public_space));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let mut out: Vec<PublicSpace> = by_id.into_values().map(|(_, ps)| ps).collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn parse_openbare_ruimte<B: BufRead>(
    reader: &mut Reader<B>,
    buf: &mut Vec<u8>,
    reference_date: &str,
) -> Result<Option<(u32, PublicSpace)>, quick_xml::Error> {
    let mut id = None;
    let mut name = None;
    let mut locality_id = None;
    let mut issued = false;
    let mut state = VoorkomenState::default();

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) if e.name().as_ref() == ID_TAG => {
                if let Some(value) = read_simple_tag(reader, ID_TAG, buf)? {
                    id = Some(value);
                }
            }
            Event::Start(e) if e.name().as_ref() == NAME_TAG => {
                if let Some(value) = read_simple_tag(reader, NAME_TAG, buf)? {
                    name = Some(value);
                }
            }
            Event::Start(e) if e.name().as_ref() == LOCALITY_REF_TAG => {
                if let Some(value) = read_simple_tag(reader, LOCALITY_REF_TAG, buf)? {
                    locality_id = Some(value.parse().expect("Failed to parse locality id"));
                }
            }
            Event::Start(e) if e.name().as_ref() == STATUS_TAG => {
                if let Some(value) = read_simple_tag(reader, STATUS_TAG, buf)?
                    && value == ISSUED_STATUS
                {
                    issued = true;
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
            Event::End(e) if e.name().as_ref() == OPR_TAG => break,
            Event::Eof => break,
            _ => {}
        }
    }

    if !issued || state.is_inactive(reference_date) {
        return Ok(None);
    }

    match (id, name, locality_id) {
        (Some(id), Some(name), Some(locality_id)) => Ok(Some((
            state.voorkomen_id.unwrap_or(0),
            PublicSpace {
                id,
                name,
                locality_id,
            },
        ))),
        _ => Ok(None),
    }
}
