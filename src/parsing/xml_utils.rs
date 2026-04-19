use std::io::BufRead;

use quick_xml::{escape::resolve_predefined_entity, events::Event, reader::Reader};

/// Per-voorkomen lifecycle signals collected while streaming a BAG object.
///
/// A voorkomen is outside the active lifecycle when any of these hold:
/// - `eind_geldigheid` is set (this version is superseded materially),
/// - `tijdstip_inactief` or `tijdstip_nietbag` is set (per spec §2.2.5),
/// - `begin_geldigheid` is in the future relative to the extract date.
#[derive(Default)]
pub(crate) struct VoorkomenState {
    pub eind_geldigheid: bool,
    pub tijdstip_inactief: bool,
    pub tijdstip_nietbag: bool,
    pub begin_geldigheid: Option<String>,
    pub voorkomen_id: Option<u32>,
}

impl VoorkomenState {
    /// Returns true when the voorkomen is outside the active lifecycle as of
    /// `reference_date` (YYYY-MM-DD). Dates in ISO-8601 sort lexicographically.
    pub fn is_inactive(&self, reference_date: &str) -> bool {
        if self.eind_geldigheid || self.tijdstip_inactief || self.tijdstip_nietbag {
            return true;
        }
        matches!(self.begin_geldigheid.as_deref(), Some(b) if b > reference_date)
    }
}

pub(crate) const END_VALIDITY_TAG: &[u8] = b"Historie:eindGeldigheid";
pub(crate) const BEGIN_VALIDITY_TAG: &[u8] = b"Historie:beginGeldigheid";
pub(crate) const TIJDSTIP_INACTIEF_TAG: &[u8] = b"Historie:tijdstipInactief";
pub(crate) const TIJDSTIP_NIETBAG_TAG: &[u8] = b"Historie:tijdstipNietBAG";
pub(crate) const VOORKOMEN_ID_TAG: &[u8] = b"Historie:voorkomenidentificatie";

/// Read the text content of an element, stopping at its end tag.
///
/// Entity references (`&#xeb;`, `&apos;`, …) are emitted by quick-xml as
/// separate [`Event::GeneralRef`] events that split the surrounding text.
/// Accumulate every text/cdata segment and resolve each reference so names
/// containing characters like `ë` round-trip intact (e.g. `1e Exloërmond`).
pub(crate) fn read_simple_tag<B: BufRead>(
    reader: &mut Reader<B>,
    end: &[u8],
    buf: &mut Vec<u8>,
) -> Result<Option<String>, quick_xml::Error> {
    let mut content: Option<String> = None;

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Text(t) => content
                .get_or_insert_with(String::new)
                .push_str(&t.decode()?),
            Event::CData(t) => content
                .get_or_insert_with(String::new)
                .push_str(&t.decode()?),
            Event::GeneralRef(r) => {
                if let Some(ch) = r.resolve_char_ref()? {
                    content.get_or_insert_with(String::new).push(ch);
                } else if let Some(expanded) = resolve_predefined_entity(&r.decode()?) {
                    content.get_or_insert_with(String::new).push_str(expanded);
                }
            }
            Event::End(e) if e.name().as_ref() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(content)
}
