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
    pub step: u8,
}

pub struct Database {
    pub localities: Vec<String>,
    /// BAG woonplaatsidentificatiecode per locality_index.
    pub locality_codes: Vec<u16>,
    pub public_spaces: Vec<String>,
    pub ranges: Vec<NumberRange>,
    pub municipalities: Vec<String>,
    pub provinces: Vec<String>,
    pub municipality_codes: Vec<u16>,
    /// Maps locality_index -> municipality_index (u16::MAX = unknown).
    pub locality_municipality: Vec<u16>,
    /// Maps municipality_index -> province_index.
    pub municipality_province: Vec<u8>,
    /// Parallel to `localities`: true when the source name carried a stripped
    /// province suffix. Forces `unique = false` in the JSON output.
    pub locality_had_suffix: Vec<bool>,
    /// Parallel to `municipalities`: same semantic as above for CBS entries.
    pub municipality_had_suffix: Vec<bool>,
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
    municipality_count: u32,
    province_count: u32,
    municipality_offsets_offset: usize,
    municipality_data_offset: usize,
    municipality_data_end: usize,
    province_offsets_offset: usize,
    province_data_offset: usize,
    province_data_end: usize,
    locality_municipality_map_offset: usize,
    municipality_province_map_offset: usize,
    municipality_codes_offset: usize,
    locality_codes_offset: usize,
    locality_had_suffix_offset: usize,
    municipality_had_suffix_offset: usize,
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

    /// Iterate over all localities, yielding (name, locality_code, municipality_name, municipality_code, province_code, name_unique, had_suffix).
    ///
    /// `locality_code` is the BAG woonplaatsidentificatiecode, which uniquely
    /// identifies the Woonplaats even when names are shared across entries.
    /// `name_unique` is true when this locality's name appears only here
    /// across all localities and municipalities; a locality sharing its name
    /// with its own parent municipality is not treated as a collision.
    /// `had_suffix` is true when the source BAG name carried a stripped
    /// province suffix (e.g. `Loo Gld` → `Loo`).
    pub fn locality_details(&self) -> Vec<(&str, u16, &str, u16, &str, bool, bool)> {
        match self {
            DatabaseHandle::Decoded(db) => db.locality_details(),
            DatabaseHandle::View(view) => view.locality_details(),
        }
    }

    /// Iterate over all municipalities, yielding (name, code, province_name, name_unique, had_suffix).
    ///
    /// `had_suffix` is true when the CBS name carried a stripped province
    /// suffix (e.g. `Hengelo (O.)` → `Hengelo`).
    pub fn municipality_details(&self) -> Vec<(&str, u16, &str, bool, bool)> {
        match self {
            DatabaseHandle::Decoded(db) => db.municipality_details(),
            DatabaseHandle::View(view) => view.municipality_details(),
        }
    }

    /// Fuzzy-search localities and municipalities for `query`.
    ///
    /// See [`crate::suggest::suggest`] for the scoring details.
    pub fn suggest(
        &self,
        query: &str,
        threshold: f32,
        limit: usize,
    ) -> Vec<crate::suggest::SuggestEntry> {
        crate::suggest::suggest(self, query, threshold, limit)
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
        let db_path = PathBuf::from("test/bag.bin");

        let db_bytes = std::fs::read(&db_path).unwrap();
        let mut decoder = GzDecoder::new(&db_bytes[..]);
        let db = Database::from_reader(&mut decoder).unwrap();

        verify_test_db(&db);
    }
}
