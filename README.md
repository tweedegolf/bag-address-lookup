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

Suggest localities by prefix or fuzzy match:

```sh
curl "http://127.0.0.1:8080/suggest?wp=Amster"
```

Example response:

```json
["Amsterdam","Amstelveen"]
```

If the `wp` query param is missing, the service responds with `400` and:

```json
{"error":"missing wp"}
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

The `create-db` binary downloads the official BAG extract from Kadaster. It
streams through the ZIP, parses only the required XML elements, and discards
everything else. The resulting data is written to disk in a compact binary
layout, `data/bag.bin`, optimized for fast lookup and low memory use.
This binary is loaded into the resulting application at compile time.

## Binary format

All integers are little-endian.

| Offset | Size             | Field                       | Description                            |
|--------|------------------|-----------------------------|----------------------------------------|
| 0      | 4                | magic header                | `BAG1`                                 |
| 4      | 4                | locality_count              | number of locality names               |
| 8      | 4                | public_space_count          | number of street names                 |
| 12     | 4                | range_count                 | number of address ranges               |
| 16     | 4                | locality_offsets_offset     | start of locality offsets array        |
| 20     | 4                | locality_data_offset        | start of locality name bytes           |
| 24     | 4                | public_space_offsets_offset | start of public space offsets array    |
| 28     | 4                | public_space_data_offset    | start of public space name bytes       |
| 32     | 4                | ranges_offset               | start of range records                 |
| ...    | ...              | locality_offsets            | `(locality_count + 1)` u32 offsets     |
| ...    | ...              | locality_data               | concatenated locality bytes            |
| ...    | ...              | public_space_offsets        | `(public_space_count + 1)` u32 offsets |
| ...    | ...              | public_space_data           | concatenated public space bytes        |
| ...    | 16 * range_count | ranges                      | range records                          |

Range record (16 bytes):

| Field              | Size | Description                  |
|--------------------|------|------------------------------|
| postal_code        | 4    | encoded postal code          |
| start              | 4    | first house number in range  |
| length             | 2    | number of addresses in range |
| public_space_index | 4    | index into public_space list |
| locality_index     | 2    | index into locality list     |

By default the `bag.bin` file is stored compressed with gzip. At startup, the web service
stream-decompresses it and decodes the data into:
- `Vec<String>` for localities
- `Vec<String>` for public spaces
- `Vec<NumberRange>` for address ranges

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

- BAG: https://www.kadaster.nl/-/gratis-download-bag-extract  
- Download: https://service.pdok.nl/kadaster/adressen/atom/v1_0/downloads/lvbag-extract-nl.zip  
