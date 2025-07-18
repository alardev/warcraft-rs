use criterion::{Criterion, criterion_group, criterion_main};
use std::io::Cursor;
use wow_m2::converter::M2Converter;
use wow_m2::header::M2Header;
use wow_m2::model::M2Model;
use wow_m2::version::M2Version;

fn create_test_model() -> M2Model {
    // Create a simple model for testing
    let header = M2Header::new(M2Version::Classic);

    M2Model {
        header,
        name: Some("TestModel".to_string()),
        global_sequences: vec![0, 100, 200],
        animations: vec![],
        animation_lookup: vec![],
        bones: vec![],
        key_bone_lookup: vec![],
        vertices: vec![],
        textures: vec![],
        materials: vec![],
        raw_data: Default::default(),
    }
}

fn bench_model_parse(c: &mut Criterion) {
    // Create a test model
    let model = create_test_model();

    // Convert to a byte array
    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);
    model.write(&mut cursor).unwrap();

    c.bench_function("parse_model", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(&data);
            let _model = M2Model::parse(&mut cursor).unwrap();
        })
    });
}

fn bench_model_convert(c: &mut Criterion) {
    // Create a test model
    let model = create_test_model();
    let converter = M2Converter::new();

    c.bench_function("convert_model", |b| {
        b.iter(|| {
            let _converted = converter.convert(&model, M2Version::Cataclysm).unwrap();
        })
    });
}

criterion_group!(benches, bench_model_parse, bench_model_convert);
criterion_main!(benches);
