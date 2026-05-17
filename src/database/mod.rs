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

/// Details for one locality, as returned by [`DatabaseHandle::locality_details`].
#[derive(Debug, Clone, Copy)]
pub struct LocalityDetail<'a> {
    /// Locality (woonplaats) name.
    pub name: &'a str,
    /// BAG woonplaatsidentificatiecode — uniquely identifies the woonplaats
    /// even when names are shared across entries.
    pub code: u16,
    /// Name of the municipality this locality belongs to.
    pub municipality: &'a str,
    /// BAG gemeentecode of that municipality.
    pub municipality_code: u16,
    /// Two-letter province code of that municipality.
    pub province: &'a str,
    /// True when this name appears only here across all localities and
    /// municipalities; sharing a name with the parent municipality does not
    /// count as a collision.
    pub unique: bool,
    /// True when the source BAG name carried a stripped province suffix
    /// (e.g. `Loo Gld` → `Loo`).
    pub had_suffix: bool,
}

/// Details for one municipality, as returned by
/// [`DatabaseHandle::municipality_details`].
#[derive(Debug, Clone, Copy)]
pub struct MunicipalityDetail<'a> {
    /// Municipality (gemeente) name.
    pub name: &'a str,
    /// BAG gemeentecode.
    pub code: u16,
    /// Two-letter province code.
    pub province: &'a str,
    /// True when this name appears only here across all localities and
    /// municipalities.
    pub unique: bool,
    /// True when the CBS name carried a stripped province suffix
    /// (e.g. `Hengelo (O.)` → `Hengelo`).
    pub had_suffix: bool,
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

    /// Return details for every locality that has a known municipality.
    ///
    /// See [`LocalityDetail`] for the meaning of each field.
    pub fn locality_details(&self) -> Vec<LocalityDetail<'_>> {
        match self {
            DatabaseHandle::Decoded(db) => db.locality_details(),
            DatabaseHandle::View(view) => view.locality_details(),
        }
    }

    /// Return details for every municipality.
    ///
    /// See [`MunicipalityDetail`] for the meaning of each field.
    pub fn municipality_details(&self) -> Vec<MunicipalityDetail<'_>> {
        match self {
            DatabaseHandle::Decoded(db) => db.municipality_details(),
            DatabaseHandle::View(view) => view.municipality_details(),
        }
    }

    /// Fuzzy-search localities and municipalities for `query`, returning the
    /// matching names.
    ///
    /// When `include_municipalities` is false, municipality names are omitted.
    /// When `include_aliases` is false, locality aliases are omitted.
    ///
    /// See [`crate::suggest::suggest`] for the scoring details.
    pub fn suggest(
        &self,
        query: &str,
        threshold: f32,
        limit: usize,
        include_municipalities: bool,
        include_aliases: bool,
    ) -> Vec<String> {
        crate::suggest::suggest(
            self,
            query,
            threshold,
            limit,
            include_municipalities,
            include_aliases,
        )
    }

    /// Load the embedded BAG database.
    pub fn load() -> Result<DatabaseHandle, DatabaseError> {
        #[cfg(feature = "compressed_database")]
        {
            let mut decoder =
                zstd::Decoder::new(DATABASE_BYTES).map_err(|_| DatabaseError::InvalidMagic)?;
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
        let mut decoder = zstd::Decoder::new(&db_bytes[..]).unwrap();
        let db = Database::from_reader(&mut decoder).unwrap();

        verify_test_db(&db);
    }
}
