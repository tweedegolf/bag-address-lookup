use crate::{
    Database, LocalityMap, encode_addresses, index_localities, index_public_spaces,
    parsing::ParsedData,
};

impl Database {
    /// Build a database from parsed BAG data.
    pub fn from_parsed_data(data: ParsedData) -> Result<Database, Box<dyn std::error::Error>> {
        let ParsedData {
            addresses,
            public_spaces,
            localities,
        } = data;

        let LocalityMap {
            locality_names,
            locality_map,
        } = index_localities(localities)?;

        let (pc_names, ps_map) = index_public_spaces(public_spaces, locality_map);
        let ranges = encode_addresses(addresses, &ps_map);

        Ok(Database {
            localities: locality_names,
            public_spaces: pc_names,
            ranges,
        })
    }
}
