//! tests/parser_tests.rs - Tests for the ADT parser

use pretty_assertions::assert_eq;
use std::io::Cursor;
use wow_adt::{Adt, AdtVersion, ChunkHeader, MverChunk};

#[test]
fn test_chunk_header_parsing() {
    // Create a simple chunk header
    let mut data = Vec::new();
    data.extend_from_slice(b"REVM"); // Magic (stored reversed in file)
    data.extend_from_slice(&[4, 0, 0, 0]); // Size (4 bytes, little endian)

    let mut cursor = Cursor::new(data);

    // Parse the header
    let header = ChunkHeader::read(&mut cursor).unwrap();

    // Validate the result
    assert_eq!(header.magic, *b"MVER"); // After reversing during read
    assert_eq!(header.size, 4);
    assert_eq!(header.magic_as_string(), "MVER");
}

#[test]
fn test_mver_parsing() {
    // Create a simple MVER chunk
    let mut data = Vec::new();
    data.extend_from_slice(b"REVM"); // Magic (stored reversed in file)
    data.extend_from_slice(&[4, 0, 0, 0]); // Size (4 bytes, little endian)
    data.extend_from_slice(&[18, 0, 0, 0]); // Version (18, standard for ADT)

    let mut cursor = Cursor::new(data);

    // Parse the MVER chunk
    let mver = MverChunk::read(&mut cursor).unwrap();

    // Validate the result
    assert_eq!(mver.version, 18);
}

#[test]
fn test_version_detection() {
    // Test converting various MVER values to AdtVersion
    let vanilla = AdtVersion::from_mver(18).unwrap();
    assert_eq!(vanilla, AdtVersion::Vanilla);

    // Test error for invalid version
    let invalid = AdtVersion::from_mver(42);
    assert!(invalid.is_err());
}

#[test]
fn test_version_comparison() {
    // Test version ordering
    assert!(AdtVersion::Vanilla < AdtVersion::TBC);
    assert!(AdtVersion::TBC < AdtVersion::WotLK);
    assert!(AdtVersion::WotLK < AdtVersion::Cataclysm);
    assert!(AdtVersion::Cataclysm < AdtVersion::MoP);
    assert!(AdtVersion::MoP < AdtVersion::WoD);
    assert!(AdtVersion::WoD < AdtVersion::Legion);
    assert!(AdtVersion::Legion < AdtVersion::BfA);
    assert!(AdtVersion::BfA < AdtVersion::Shadowlands);
    assert!(AdtVersion::Shadowlands < AdtVersion::Dragonflight);
}

#[test]
fn test_empty_adt_creation() {
    // Create a minimal ADT with just a MVER chunk
    let mut data = Vec::new();
    data.extend_from_slice(b"REVM"); // Magic (stored reversed in file)
    data.extend_from_slice(&[4, 0, 0, 0]); // Size (4 bytes, little endian)
    data.extend_from_slice(&[18, 0, 0, 0]); // Version (18)

    let mut cursor = Cursor::new(data);

    // Parse the ADT
    let adt = Adt::from_reader(&mut cursor);

    // Since this isn't a complete ADT (missing MHDR, etc.), it should fail to parse
    assert!(adt.is_err());
    // The error should be about the file being too small
    if let Err(e) = adt {
        let error_msg = format!("{:?}", e);
        assert!(error_msg.contains("too small") || error_msg.contains("InvalidFileSize"));
    }
}

#[test]
fn test_version_to_string() {
    assert_eq!(AdtVersion::Vanilla.to_string(), "Vanilla (1.x)");
    assert_eq!(AdtVersion::TBC.to_string(), "The Burning Crusade (2.x)");
    assert_eq!(
        AdtVersion::WotLK.to_string(),
        "Wrath of the Lich King (3.x)"
    );
    assert_eq!(AdtVersion::Cataclysm.to_string(), "Cataclysm (4.x)");
}
