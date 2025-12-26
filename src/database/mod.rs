#[cfg(feature = "create")]
mod create;

#[cfg(feature = "create")]
mod encode;

mod decode;
mod error;
mod layout;
mod lookup;
mod rw;
mod util;
mod view;

pub use error::DatabaseError;
pub use util::encode_pc;

pub struct NumberRange {
    pub postal_code: u32,
    pub start: u32,
    pub length: u16,
    pub public_space_index: u32,
    pub locality_index: u16,
}

pub struct Database {
    pub localities: Vec<String>,
    pub public_spaces: Vec<String>,
    pub ranges: Vec<NumberRange>,
}

pub struct DatabaseView {
    bytes: &'static [u8],
    locality_count: u32,
    public_space_count: u32,
    range_count: u32,
    locality_offsets_offset: usize,
    locality_data_offset: usize,
    locality_data_end: usize,
    public_space_offsets_offset: usize,
    public_space_data_offset: usize,
    public_space_data_end: usize,
    ranges_offset: usize,
}

#[cfg(not(feature = "create"))]
pub(crate) const DATABASE_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/bag.bin"));

#[cfg(feature = "create")]
pub(crate) const DATABASE_BYTES: &[u8] = &[];

pub enum DatabaseHandle {
    Decoded(Database),
    View(DatabaseView),
}

pub struct Localities<'a> {
    inner: LocalitiesInner<'a>,
}

enum LocalitiesInner<'a> {
    Decoded(std::slice::Iter<'a, String>),
    View { view: &'a DatabaseView, index: u32 },
}

impl<'a> Iterator for Localities<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            LocalitiesInner::Decoded(iter) => iter.next().map(String::as_str),
            LocalitiesInner::View { view, index } => {
                if *index > u16::MAX as u32 {
                    return None;
                }
                while *index < view.locality_count {
                    let current = *index;
                    *index += 1;
                    if current > u16::MAX as u32 {
                        return None;
                    }
                    if let Some(name) = view.locality_name(current as u16) {
                        return Some(name);
                    }
                }
                None
            }
        }
    }
}

impl DatabaseHandle {
    pub fn is_empty(&self) -> bool {
        match self {
            DatabaseHandle::Decoded(db) => db.is_empty(),
            DatabaseHandle::View(view) => view.is_empty(),
        }
    }

    pub fn localities(&'_ self) -> Localities<'_> {
        match self {
            DatabaseHandle::Decoded(db) => Localities {
                inner: LocalitiesInner::Decoded(db.localities.iter()),
            },
            DatabaseHandle::View(view) => Localities {
                inner: LocalitiesInner::View { view, index: 0 },
            },
        }
    }

    pub fn lookup(&self, postalcode: &str, house_number: u32) -> Option<(&str, &str)> {
        match self {
            DatabaseHandle::Decoded(db) => db.lookup(postalcode, house_number),
            DatabaseHandle::View(view) => view.lookup(postalcode, house_number),
        }
    }

    /// Load the embedded BAG database.
    pub fn load() -> Result<DatabaseHandle, DatabaseError> {
        #[cfg(feature = "compressed_database")]
        {
            use flate2::bufread::GzDecoder;

            let mut decoder = GzDecoder::new(DATABASE_BYTES);
            let db = Database::from_reader(&mut decoder)?;
            Ok(DatabaseHandle::Decoded(db))
        }
        #[cfg(not(feature = "compressed_database"))]
        {
            let view = DatabaseView::from_bytes(DATABASE_BYTES)?;
            Ok(DatabaseHandle::View(view))
        }
    }
}

#[cfg(all(test, feature = "compressed_database"))]
mod tests {
    use flate2::bufread::GzDecoder;
    use std::path::PathBuf;

    use super::*;

    fn verify_test_db(db: &Database) {
        assert_eq!(db.localities.len(), 2);
        assert_eq!(db.public_spaces.len(), 2);
        assert_eq!(db.ranges.len(), 2);

        let lookup_result = db.lookup("1234AB", 56).unwrap();
        assert_eq!(lookup_result.0, "Abel Eppensstraat");
        assert_eq!(lookup_result.1, "Hoogerheide");

        let lookup_result = db.lookup("1234AB", 1).unwrap();
        assert_eq!(lookup_result.0, "Adamistraat");
        assert_eq!(lookup_result.1, "Huijbergen");

        let lookup_none = db.lookup("9999ZZ", 1);
        assert!(lookup_none.is_none());
    }

    #[test]
    fn test_decode_db() {
        #[cfg(feature = "compressed_database")]
        let db_path = PathBuf::from("test/bag.bin");

        #[cfg(not(feature = "compressed_database"))]
        let db_path = PathBuf::from("test/bag_uncompressed.bin");

        let db_bytes = std::fs::read(&db_path).unwrap();
        let mut decoder = GzDecoder::new(&db_bytes[..]);
        let db = Database::from_reader(&mut decoder).unwrap();

        verify_test_db(&db);
    }
}
