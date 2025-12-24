use std::io::BufRead;

use quick_xml::{events::Event, reader::Reader};

use super::xml_utils::read_simple_tag;

const WP_TAG: &[u8] = b"Objecten:Woonplaats";
const ID_TAG: &[u8] = b"Objecten:identificatie";
const NAME_TAG: &[u8] = b"Objecten:naam";
const END_VALIDITY_TAG: &[u8] = b"Historie:eindGeldigheid";

#[derive(Debug, PartialEq, Eq)]
pub struct Locality {
    pub id: u16,
    pub name: String,
}

/// Parse BAG locality XML data into structured locality records.
pub fn parse_localities<R: std::io::BufRead>(reader: R) -> Result<Vec<Locality>, quick_xml::Error> {
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut localities = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == WP_TAG => {
                if let Some(locality) = parse_woonplaats(&mut reader, &mut buf)? {
                    localities.push(locality);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(localities)
}

fn parse_woonplaats<B: BufRead>(
    reader: &mut Reader<B>,
    buf: &mut Vec<u8>,
) -> Result<Option<Locality>, quick_xml::Error> {
    let mut id = None;
    let mut name = None;
    let mut expired = false;

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
            Event::Start(e) if e.name().as_ref() == END_VALIDITY_TAG => {
                expired = true;
                let _ = read_simple_tag(reader, END_VALIDITY_TAG, buf)?;
            }
            Event::End(e) if e.name().as_ref() == WP_TAG => break,
            Event::Eof => break,
            _ => {}
        }
    }

    if expired {
        return Ok(None);
    }

    match (id, name) {
        (Some(id), Some(name)) => Ok(Some(Locality { id, name })),
        _ => Ok(None),
    }
}
