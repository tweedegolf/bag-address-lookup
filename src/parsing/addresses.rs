// Parses Nummeraanduiding (address designation) objects from the BAG extract.
// BAG catalog §7.4: https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag/catalogus-bag
//
// A Nummeraanduiding assigns a house number and postal code to an addressable
// object via an OpenbareRuimte. Only currently valid records with status
// "Naamgeving uitgegeven" are included.

use std::io::BufRead;

use quick_xml::{Reader, events::Event};

use super::xml_utils::read_simple_tag;

const NUM_TAG: &[u8] = b"Objecten:Nummeraanduiding";
// §7.4.1 identificatie - 16-digit national identifier
const ID_TAG: &[u8] = b"Objecten:identificatie";
// §7.4.2 huisnummer - house number (1-99999)
const HOUSE_NUMBER_TAG: &[u8] = b"Objecten:huisnummer";
// §7.4.5 postcode - 6-character Dutch postal code (e.g. "1234AB")
const POSTAL_CODE_TAG: &[u8] = b"Objecten:postcode";
// §7.4.8 ligtAan - reference to the OpenbareRuimte this address belongs to
const PUBLIC_SPACE_REF_TAG: &[u8] = b"Objecten-ref:OpenbareRuimteRef";
// §7.4.10 tijdvakGeldigheid/eindGeldigheid - presence means this version is superseded
const END_VALIDITY_TAG: &[u8] = b"Historie:eindGeldigheid";
// §7.4.7 status - lifecycle status of the address designation
const STATUS_TAG: &[u8] = b"Objecten:status";
// Only include addresses where a name/number has been officially issued
const ISSUED_STATUS: &str = "Naamgeving uitgegeven";

#[derive(Debug, PartialEq, Eq)]
pub struct Address {
    pub id: String,
    pub house_number: u32,
    pub postal_code: String,
    pub public_space_id: String,
}

/// Parse BAG address XML data into structured address records.
pub fn parse_addresses<R: std::io::BufRead>(source: R) -> Result<Vec<Address>, quick_xml::Error> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut addresses = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == NUM_TAG => {
                if let Some(address) = parse_address(&mut reader, &mut buf)? {
                    addresses.push(address);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(addresses)
}

fn parse_address<B: BufRead>(
    reader: &mut Reader<B>,
    buf: &mut Vec<u8>,
) -> Result<Option<Address>, quick_xml::Error> {
    let mut id = None;
    let mut house_number = None;
    let mut postal_code = None;
    let mut public_space_id = None;
    let mut expired = false;
    let mut issued = false;
    let mut invalid = None;

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) if e.name().as_ref() == ID_TAG => {
                if let Some(value) = read_simple_tag(reader, ID_TAG, buf)? {
                    id = Some(value.parse().expect("Failed to parse address id"));
                }
            }
            Event::Start(e) if e.name().as_ref() == HOUSE_NUMBER_TAG => {
                if let Some(value) = read_simple_tag(reader, HOUSE_NUMBER_TAG, buf)? {
                    if let Ok(num) = value.parse::<u32>() {
                        house_number = Some(num);
                    } else {
                        invalid = Some(value);
                    }
                }
            }
            Event::Start(e) if e.name().as_ref() == POSTAL_CODE_TAG => {
                if let Some(value) = read_simple_tag(reader, POSTAL_CODE_TAG, buf)? {
                    postal_code = Some(value);
                }
            }
            Event::Start(e) if e.name().as_ref() == PUBLIC_SPACE_REF_TAG => {
                if let Some(value) = read_simple_tag(reader, PUBLIC_SPACE_REF_TAG, buf)? {
                    public_space_id = Some(value);
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
            Event::End(e) if e.name().as_ref() == NUM_TAG => break,
            Event::Eof => break,
            _ => {}
        }
    }

    if expired || !issued {
        return Ok(None);
    }

    if let Some(invalid_value) = invalid {
        eprintln!(
            "Warning: Skipping address with invalid house number '{}'",
            invalid_value
        );
        return Ok(None);
    }

    match (id, house_number, postal_code, public_space_id) {
        (Some(id), Some(house_number), Some(postal_code), Some(public_space_id)) => {
            Ok(Some(Address {
                id,
                house_number,
                postal_code,
                public_space_id,
            }))
        }
        _ => Ok(None),
    }
}
