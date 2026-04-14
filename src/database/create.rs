use crate::{
    Database, LocalityMap, MunicipalityMap, build_locality_province_map, encode_addresses,
    index_localities, index_municipalities, index_public_spaces,
    parsing::{ParsedData, municipalities::Municipality},
};

impl Database {
    /// Build a database from parsed BAG data and CBS municipality data.
    pub fn from_parsed_data(
        data: ParsedData,
        cbs_municipalities: &[Municipality],
    ) -> Result<Database, Box<dyn std::error::Error>> {
        let ParsedData {
            addresses,
            public_spaces,
            localities,
            municipality_relations,
        } = data;

        let locality_province =
            build_locality_province_map(&municipality_relations, cbs_municipalities);

        let LocalityMap {
            locality_names,
            locality_map,
        } = index_localities(localities, &locality_province)?;

        let MunicipalityMap {
            municipality_names,
            province_names,
            municipality_codes,
            locality_municipality,
            municipality_province,
        } = index_municipalities(
            municipality_relations,
            cbs_municipalities,
            &locality_map,
            locality_names.len(),
        )?;

        let (pc_names, ps_map) = index_public_spaces(public_spaces, locality_map);
        let ranges = encode_addresses(addresses, &ps_map);

        Ok(Database {
            localities: locality_names,
            public_spaces: pc_names,
            ranges,
            municipalities: municipality_names,
            provinces: province_names,
            municipality_codes,
            locality_municipality,
            municipality_province,
        })
    }
}
