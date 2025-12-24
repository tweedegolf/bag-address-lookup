use std::io::BufRead;

use quick_xml::{events::Event, reader::Reader};

use super::xml_utils::read_simple_tag;

const OPR_TAG: &[u8] = b"Objecten:OpenbareRuimte";
const ID_TAG: &[u8] = b"Objecten:identificatie";
const NAME_TAG: &[u8] = b"Objecten:naam";
const LOCALITY_REF_TAG: &[u8] = b"Objecten-ref:WoonplaatsRef";
const END_VALIDITY_TAG: &[u8] = b"Historie:eindGeldigheid";
const STATUS_TAG: &[u8] = b"Objecten:status";
const ISSUED_STATUS: &str = "Naamgeving uitgegeven";

#[derive(Debug, PartialEq, Eq)]
pub struct PublicSpace {
    pub id: String,
    pub name: String,
    pub locality_id: u16,
}

/// Parse BAG public space XML data into structured public space records.
pub fn parse_public_spaces<R: std::io::BufRead>(
    source: R,
) -> Result<Vec<PublicSpace>, quick_xml::Error> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut public_spaces = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == OPR_TAG => {
                if let Some(public_space) = parse_openbare_ruimte(&mut reader, &mut buf)? {
                    public_spaces.push(public_space);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(public_spaces)
}

fn parse_openbare_ruimte<B: BufRead>(
    reader: &mut Reader<B>,
    buf: &mut Vec<u8>,
) -> Result<Option<PublicSpace>, quick_xml::Error> {
    let mut id = None;
    let mut name = None;
    let mut locality_id = None;
    let mut expired = false;
    let mut issued = false;

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
            Event::Start(e) if e.name().as_ref() == END_VALIDITY_TAG => {
                expired = true;
                let _ = read_simple_tag(reader, END_VALIDITY_TAG, buf)?;
            }
            Event::Start(e) if e.name().as_ref() == STATUS_TAG => {
                if let Some(value) = read_simple_tag(reader, STATUS_TAG, buf)?
                    && value == ISSUED_STATUS
                {
                    issued = true;
                }
            }
            Event::End(e) if e.name().as_ref() == OPR_TAG => break,
            Event::Eof => break,
            _ => {}
        }
    }

    if expired || !issued {
        return Ok(None);
    }

    match (id, name, locality_id) {
        (Some(id), Some(name), Some(locality_id)) => Ok(Some(PublicSpace {
            id,
            name,
            locality_id,
        })),
        _ => Ok(None),
    }
}
