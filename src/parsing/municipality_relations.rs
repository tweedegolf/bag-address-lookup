// Parses Gemeente-Woonplaats-Relatie (GWR) records from the BAG extract.
//
// The GWR file maps each Woonplaats (locality) to the Gemeente (municipality) it
// belongs to. Records with an end validity date (einddatumTijdvakGeldigheid) or a
// future begin date are excluded so only relations active on the extract's
// standtechnische datum remain.

use std::{collections::HashMap, io::BufRead};

use quick_xml::{events::Event, reader::Reader};

use super::xml_utils::read_simple_tag;

const GWR_TAG: &[u8] = b"gwr-product:GemeenteWoonplaatsRelatie";
const RELATED_WP_TAG: &[u8] = b"gwr-product:gerelateerdeWoonplaats";
const RELATED_GM_TAG: &[u8] = b"gwr-product:gerelateerdeGemeente";
const ID_TAG: &[u8] = b"gwr-product:identificatie";
const BEGIN_VALIDITY_TAG: &[u8] = b"bagtypes:begindatumTijdvakGeldigheid";
const END_VALIDITY_TAG: &[u8] = b"bagtypes:einddatumTijdvakGeldigheid";

#[derive(Debug, PartialEq, Eq)]
pub struct MunicipalityRelation {
    pub locality_id: u16,
    pub municipality_code: u16,
}

/// Parse GWR XML data into municipality-locality relation records.
///
/// `reference_date` is the extract's standtechnische datum (YYYY-MM-DD).
/// Relations with a future begin date are excluded. If a locality appears in
/// multiple current relations, the one parsed latest wins (consistent with
/// how BAG deliveries order chronological voorkomens).
pub fn parse_municipality_relations<R: BufRead>(
    reader: R,
    reference_date: &str,
) -> Result<Vec<MunicipalityRelation>, quick_xml::Error> {
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut by_locality: HashMap<u16, u16> = HashMap::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == GWR_TAG => {
                if let Some(relation) = parse_relation(&mut reader, &mut buf, reference_date)? {
                    by_locality.insert(relation.locality_id, relation.municipality_code);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let mut out: Vec<MunicipalityRelation> = by_locality
        .into_iter()
        .map(|(locality_id, municipality_code)| MunicipalityRelation {
            locality_id,
            municipality_code,
        })
        .collect();
    out.sort_by_key(|r| r.locality_id);
    Ok(out)
}

fn parse_relation<B: BufRead>(
    reader: &mut Reader<B>,
    buf: &mut Vec<u8>,
    reference_date: &str,
) -> Result<Option<MunicipalityRelation>, quick_xml::Error> {
    let mut locality_id = None;
    let mut municipality_code = None;
    let mut expired = false;
    let mut begin_geldigheid: Option<String> = None;

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) if e.name().as_ref() == RELATED_WP_TAG => {
                locality_id = parse_nested_id(reader, RELATED_WP_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == RELATED_GM_TAG => {
                municipality_code = parse_nested_id(reader, RELATED_GM_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == BEGIN_VALIDITY_TAG => {
                begin_geldigheid = read_simple_tag(reader, BEGIN_VALIDITY_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == END_VALIDITY_TAG => {
                expired = true;
                let _ = read_simple_tag(reader, END_VALIDITY_TAG, buf)?;
            }
            Event::End(e) if e.name().as_ref() == GWR_TAG => break,
            Event::Eof => break,
            _ => {}
        }
    }

    if expired {
        return Ok(None);
    }
    if let Some(b) = begin_geldigheid.as_deref() {
        if b > reference_date {
            return Ok(None);
        }
    }

    match (locality_id, municipality_code) {
        (Some(lid), Some(mc)) => Ok(Some(MunicipalityRelation {
            locality_id: lid,
            municipality_code: mc,
        })),
        _ => Ok(None),
    }
}

/// Read a `gwr-product:identificatie` value nested inside a parent element.
fn parse_nested_id<B: BufRead>(
    reader: &mut Reader<B>,
    parent_end: &[u8],
    buf: &mut Vec<u8>,
) -> Result<Option<u16>, quick_xml::Error> {
    let mut id = None;

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) if e.name().as_ref() == ID_TAG => {
                if let Some(value) = read_simple_tag(reader, ID_TAG, buf)? {
                    id = Some(value.parse().expect("Failed to parse GWR identificatie"));
                }
            }
            Event::End(e) if e.name().as_ref() == parent_end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(id)
}
