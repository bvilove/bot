use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct City {
    county: String,
    subject: String,
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Subject {
    name: String,
    id: i32,
}

#[derive(Debug, Deserialize)]
struct County {
    name: String,
    id: i32,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cities.csv");
    println!("cargo:rerun-if-changed=subjects.csv");
    println!("cargo:rerun-if-changed=counties.csv");

    let citiesmap_path =
        Path::new(&env::var("OUT_DIR").unwrap()).join("citiesmap.rs");
    let mut citiesmap_file =
        BufWriter::new(File::create(citiesmap_path).unwrap());

    // Subjects
    let subjects_file =
        BufReader::new(File::open(Path::new("subjects.csv")).unwrap());
    let mut subjects_map = &mut phf_codegen::Map::new();
    let mut subjects_map_rev = &mut phf_codegen::Map::new();
    let mut subjects = HashMap::new();
    let mut subjects_rdr = csv::Reader::from_reader(subjects_file);
    for result in subjects_rdr.deserialize() {
        let subject: Subject = result.unwrap();
        subjects_map =
            subjects_map.entry(subject.id, &format!("\"{}\"", subject.name));
        subjects_map_rev = subjects_map_rev
            .entry(subject.name.clone(), &subject.id.to_string());
        subjects.insert(subject.name, subject.id);
    }
    write!(
        &mut citiesmap_file,
        "pub static SUBJECTS: phf::Map<i32, &'static str> = {}",
        subjects_map.build()
    )
    .unwrap();
    writeln!(&mut citiesmap_file, ";").unwrap();
    write!(
        &mut citiesmap_file,
        "pub static SUBJECTS_REV: phf::Map<&str, i32> = {}",
        subjects_map_rev.build()
    )
    .unwrap();
    writeln!(&mut citiesmap_file, ";").unwrap();

    // Counties
    let counties_file =
        BufReader::new(File::open(Path::new("counties.csv")).unwrap());
    let mut counties_map = &mut phf_codegen::Map::new();
    let mut counties_map_rev = &mut phf_codegen::Map::new();
    let mut counties = HashMap::new();
    let mut counties_rdr = csv::Reader::from_reader(counties_file);
    for result in counties_rdr.deserialize() {
        let county: County = result.unwrap();
        counties_map =
            counties_map.entry(county.id, &format!("\"{}\"", county.name));
        counties_map_rev =
            counties_map_rev.entry(county.name.clone(), &county.id.to_string());
        counties.insert(county.name, county.id);
    }
    write!(
        &mut citiesmap_file,
        "pub static COUNTIES: phf::Map<i32, &'static str> = {}",
        counties_map.build()
    )
    .unwrap();
    writeln!(&mut citiesmap_file, ";").unwrap();
    write!(
        &mut citiesmap_file,
        "pub static COUNTIES_REV: phf::Map<&str, i32> = {}",
        counties_map_rev.build()
    )
    .unwrap();
    writeln!(&mut citiesmap_file, ";").unwrap();

    let cities_file =
        BufReader::new(File::open(Path::new("cities.csv")).unwrap());
    let mut cities_map = &mut phf_codegen::Map::new();
    let mut cities_map_rev = &mut phf_codegen::Map::new();
    let mut cities_rdr = csv::Reader::from_reader(cities_file);
    for result in cities_rdr.deserialize() {
        let city: City = result.unwrap();
        let city_id = (((counties
            .get(&city.county)
            .unwrap_or_else(|| panic!("{} not found", city.county))
            << 8)
            + subjects
                .get(&city.subject)
                .unwrap_or_else(|| panic!("{} not found", city.subject)))
            << 8)
            + city.id;
        cities_map = cities_map.entry(city_id, &format!("\"{}\"", city.name));
        cities_map_rev =
            cities_map_rev.entry(city.name.clone(), &city_id.to_string());
    }
    write!(
        &mut citiesmap_file,
        "pub static CITIES: phf::Map<i32, &'static str> = {}",
        cities_map.build()
    )
    .unwrap();
    writeln!(&mut citiesmap_file, ";").unwrap();
    write!(
        &mut citiesmap_file,
        "pub static CITIES_REV: phf::Map<&str, i32> = {}",
        cities_map_rev.build()
    )
    .unwrap();
    writeln!(&mut citiesmap_file, ";").unwrap();
}
