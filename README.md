# BAG address lookup

Lookup a public space (street) name and locality (city/town) name for a given Dutch postal
code and house number. This project builds a compact, query friendly database
from the official BAG dataset and serves lookups over a small HTTP API.
The database can be created in less than a minute by streaming through the
official ZIP file and XML.

The database and HTTP lookup service are distributed as a small (<20 MB) self-contained binary.

## Purpose

Given:
- postal code (e.g. `1234AB`)
- house number (e.g. `56`)

Returns:
- street name (e.g. "Stationsstraat")
- locality name (e.g. "Amsterdam")

## Usage

Download a binary release from GitHub Releases and run it.

Service mode:

The first argument is the address to listen on, like `0.0.0.0:3000`.
This defaults to `127.0.0.1:8080`.

```sh
./bag-service 0.0.0.0:3000
```

Example request:

```sh
curl "http://127.0.0.1:8080/lookup?pc=1234AB&n=56"
```

Example response:

```json
{"pr":"Street Name","wp":"Locality"}
```

Suggest localities and municipalities by prefix or fuzzy match:

```sh
curl "http://127.0.0.1:8080/suggest?match=Amster"
```

Example response:

```json
["Amsterdam","Amstelveen"]
```

The legacy `wp` query param is still accepted as an alias for `match`.

If the `match` query param is missing, the service responds with `400` and:

```json
{"error":"missing match"}
```

List all localities with their municipality:

```sh
curl "http://127.0.0.1:8080/localities"
```

Example response:

```json
[{"wp":"Amsterdam","gm":"Amsterdam","gm_code":363},{"wp":"Amstelveen","gm":"Amstelveen","gm_code":34}]
```

List all municipalities with their province:

```sh
curl "http://127.0.0.1:8080/municipalities"
```

Example response:

```json
[{"gm":"Amsterdam","gm_code":363,"pv":"Noord-Holland"},{"gm":"Rotterdam","gm_code":599,"pv":"Zuid-Holland"}]
```

Environment variables:

- `BAG_ADDRESS_LOOKUP_QUIET=1` (or `true`) suppresses request/response logs.
- `BAG_ADDRESS_LOOKUP_SUGGEST_THRESHOLD` sets the minimum fuzzy match score for `/suggest`
  (default: `0.7`, non-negative finite float).

Lookup mode (postal code and house number arguments):

```sh
./bag-service 1234AB 56
```

Output (public space and locality, each on its own line):

```
Stationsstraat
Amsterdam
```

## How the data is built

The `create-db` binary downloads the official BAG extract from Kadaster and
municipality data from CBS (Centraal Bureau voor de Statistiek). It streams
through the BAG ZIP, parses only the required XML elements, and discards
everything else. Municipality-to-province mappings come from the CBS "Gebieden
in Nederland" table. The resulting data is written to disk in a compact binary
layout, `data/bag.bin`, optimized for fast lookup and low memory use.
This binary is loaded into the resulting application at compile time.

## Binary format

All integers are little-endian.

| Offset | Size             | Field                       | Description                            |
|--------|------------------|-----------------------------|----------------------------------------|
| 0      | 4                | magic header                | `BAG2`                                 |
| 4      | 4                | locality_count              | number of locality names               |
| 8      | 4                | public_space_count          | number of street names                 |
| 12     | 4                | range_count                 | number of address ranges               |
| 16     | 4                | locality_offsets_offset     | start of locality offsets array        |
| 20     | 4                | locality_data_offset        | start of locality name bytes           |
| 24     | 4                | public_space_offsets_offset | start of public space offsets array    |
| 28     | 4                | public_space_data_offset    | start of public space name bytes       |
| 32     | 4                | ranges_offset                    | start of range records                      |
| 36     | 4                | municipality_count               | number of municipality names                |
| 40     | 4                | province_count                   | number of province names                    |
| 44     | 4                | municipality_offsets_offset       | start of municipality offsets array         |
| 48     | 4                | municipality_data_offset         | start of municipality name bytes            |
| 52     | 4                | province_offsets_offset           | start of province offsets array             |
| 56     | 4                | province_data_offset             | start of province name bytes                |
| 60     | 4                | locality_municipality_map_offset | start of locality-to-municipality map       |
| 64     | 4                | municipality_province_map_offset | start of municipality-to-province map       |
| 68     | 4                | municipality_codes_offset        | start of municipality CBS codes             |
| ...    | ...              | locality_offsets                  | `(locality_count + 1)` u32 offsets          |
| ...    | ...              | locality_data                    | concatenated locality bytes                 |
| ...    | ...              | public_space_offsets              | `(public_space_count + 1)` u32 offsets      |
| ...    | ...              | public_space_data                 | concatenated public space bytes             |
| ...    | 17 * range_count | ranges                           | range records                               |
| ...    | ...              | municipality_offsets              | `(municipality_count + 1)` u32 offsets      |
| ...    | ...              | municipality_data                 | concatenated municipality name bytes        |
| ...    | ...              | province_offsets                  | `(province_count + 1)` u32 offsets          |
| ...    | ...              | province_data                     | concatenated province name bytes            |
| ...    | 2 * loc_count    | locality_municipality_map         | u16 municipality index per locality         |
| ...    | 1 * muni_count   | municipality_province_map         | u8 province index per municipality          |
| ...    | 2 * muni_count   | municipality_codes                | u16 CBS municipality code per municipality  |

Range record (17 bytes):

| Field              | Size | Description                                       |
|--------------------|------|---------------------------------------------------|
| postal_code        | 4    | encoded postal code                               |
| start              | 4    | first house number in range                       |
| length             | 2    | number of steps in range (0 = single address)     |
| public_space_index | 4    | index into public_space list                      |
| locality_index     | 2    | index into locality list                          |
| step               | 1    | increment between house numbers (1 or 2 typical)  |

A range covers house numbers: `start`, `start + step`, `start + 2*step`, ...,
`start + length * step`. For example, odd numbers 1-9 are encoded as
`start=1, length=4, step=2`.

By default the `bag.bin` file is stored compressed with gzip. At startup, the web service
stream-decompresses it and decodes the data into:
- `Vec<String>` for localities, public spaces, municipalities, and provinces
- `Vec<NumberRange>` for address ranges
- `Vec<u16>` / `Vec<u8>` for locality-to-municipality and municipality-to-province maps
- `Vec<u16>` for municipality CBS codes

When the `compressed_database` feature is disabled, the service performs lookups
directly against the uncompressed `bag.bin` bytes without decoding them into
vectors (zero-copy lookups).

The postal code encoding packs `1234AB` into a single `u32` for efficient
comparison and range search.

## Build the database

```sh
cargo run --release --bin create-db --features "create"
```

Disable compression and use the on-disk binary directly:

```sh
cargo run --release --bin create-db --no-default-features --features "create"
```

### Build the final release

```sh
cargo build --release --bin bag-service
```

From an uncompressed database:

```sh
cargo build --release --bin bag-service --no-default-features
```

## Sources

The [BAG](https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag) (Basisregistratie
Adressen en Gebouwen) is the Dutch national registry for addresses and buildings, maintained by
Kadaster. The full extract is published as a ZIP containing nested ZIPs with XML files following the
[StUF](https://standaarden.vng.nl/StUF-standaarden) exchange format.

The [BAG catalog](https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag/catalogus-bag)
describes all object types and their attributes. This project uses three object types
plus the Gemeente-Woonplaats relationship file:

| Object type          | File prefix    | Used attributes                                                   |
|----------------------|----------------|-------------------------------------------------------------------|
| Woonplaats           | `9999WPL`      | identificatie, naam                                               |
| OpenbareRuimte       | `9999OPR`      | identificatie, naam, WoonplaatsRef                                |
| Nummeraanduiding     | `9999NUM`      | identificatie, huisnummer, huisletter, huisnummertoevoeging, postcode, OpenbareRuimteRef |
| Gemeente-Woonplaats  | `GEM-WPL-*`    | gerelateerdeWoonplaats, gerelateerdeGemeente                      |

Only records with status "Naamgeving uitgegeven" and without an end validity date are included.

Municipality names and province mappings come from the CBS "Gebieden in Nederland" table
(OData API). The table ID is updated annually when CBS publishes a new year's edition
(see `src/parsing/municipalities.rs`).

Links:

- BAG overview: https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag
- BAG catalog (object/attribute specification): https://www.kadaster.nl/zakelijk/registraties/basisregistraties/bag/catalogus-bag
- BAG extract download: https://www.kadaster.nl/-/gratis-download-bag-extract
- Direct download (PDOK): https://service.pdok.nl/kadaster/adressen/atom/v1_0/downloads/lvbag-extract-nl.zip
- CBS municipality data: https://opendata.cbs.nl/
