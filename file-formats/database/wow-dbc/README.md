# wow-dbc

[![Crates.io](https://img.shields.io/crates/v/wow-dbc.svg)](https://crates.io/crates/wow-dbc)
[![Documentation](https://docs.rs/wow-dbc/badge.svg)](https://docs.rs/wow-dbc)
[![License](https://img.shields.io/crates/l/wow-dbc.svg)](https://github.com/wowemulation-dev/warcraft-rs#license)

Parser for World of Warcraft DBC (client database) files.

## Features

- 🔍 Parse DBC files with runtime schema definition
- 📊 Type-safe field access with proper value types
- 🔤 Efficient string block handling with caching support
- 🗂️ Indexed lookups by key field for fast access
- 🔬 Schema discovery for unknown DBC formats
- 📝 DBD (Database Definition) file support for WoWDBDefs compatibility
- 🚀 Lazy loading support for large files
- 🛠️ Export to common formats (CSV, JSON, YAML)

## Usage

```rust
use wow_dbc::{DbcParser, Schema, SchemaField, FieldType};

// Define a schema for the Map.dbc file
let mut schema = Schema::new("Map");
schema.add_field(SchemaField::new("ID", FieldType::UInt32));
schema.add_field(SchemaField::new("Directory", FieldType::String));
schema.add_field(SchemaField::new("InstanceType", FieldType::UInt32));
schema.add_field(SchemaField::new("Flags", FieldType::UInt32));
schema.add_field(SchemaField::new("MapType", FieldType::UInt32));
schema.add_field(SchemaField::new("MapName", FieldType::String));
schema.set_key_field("ID");

// Parse the DBC file
let data = std::fs::read("Map.dbc")?;
let parser = DbcParser::parse_bytes(&data)?
    .with_schema(schema)?;

let records = parser.parse_records()?;

// Access records by index
if let Some(record) = records.get_record(0) {
    if let Some(name) = record.get_value_by_name("MapName") {
        println!("Map name: {}", name);
    }
}

// Or lookup by key
if let Some(record) = records.get_record_by_key(0) {  // Eastern Kingdoms
    println!("Found map: {:?}", record);
}
```

## Supported Versions

- ✅ Classic (1.12.1) - WDBC format
- ✅ The Burning Crusade (2.4.3) - WDBC format
- ✅ Wrath of the Lich King (3.3.5a) - WDBC format
- ✅ Cataclysm (4.3.4) - WDBC format
- ✅ Mists of Pandaria (5.4.8) - WDB2/WDB5 formats

## DBD Support

This crate supports [WoWDBDefs](https://github.com/wowdev/WoWDBDefs) Database Definition files for automatic schema generation. DBD files provide community-maintained schema definitions for various WoW versions.

```rust
use wow_dbc::dbd::parse_dbd_file;

// Parse a DBD file
let dbd = parse_dbd_file("definitions/Map.dbd")?;

// Convert to schemas for different versions
let schemas = convert_to_yaml_schemas(&dbd, "Map", Some("3.3.5"), false);
```

## Tools

The crate includes several command-line tools:

- `dbc_tool` - Info, list, export, and validate DBC files
- `dbc_schema_discovery_tool` - Analyze DBC files to discover their schema
- `dbd_to_yaml` - Convert DBD definition files to YAML schemas

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](../../LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
