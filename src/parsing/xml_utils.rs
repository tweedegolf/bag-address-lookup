use std::io::BufRead;

use quick_xml::{events::Event, reader::Reader};

pub(crate) fn read_simple_tag<B: BufRead>(
    reader: &mut Reader<B>,
    end: &[u8],
    buf: &mut Vec<u8>,
) -> Result<Option<String>, quick_xml::Error> {
    let mut content = None;

    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Text(t) => content = Some(t.decode()?.into_owned()),
            Event::CData(t) => content = Some(t.decode()?.into_owned()),
            Event::End(e) if e.name().as_ref() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(content)
}
