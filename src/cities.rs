use anyhow::Context;
use itertools::Itertools;
use strsim::jaro_winkler;

include!(concat!(env!("OUT_DIR"), "/citiesmap.rs"));

pub fn find_city(query: &str) -> Option<i32> {
    let best_city = CITIES
        .entries()
        .sorted_unstable_by(|(_, left), (_, right)| {
            jaro_winkler(query, left).total_cmp(&jaro_winkler(query, right))
        })
        .next_back()
        .expect("there must be at least 1 city");
    if jaro_winkler(best_city.1, query) > 0.15 {
        Some(*best_city.0)
    } else {
        None
    }
}

// pub fn cities_list() -> String {
//     CITIES.values().sorted_unstable().map(|c| format!("{}\n", c)).collect()
// }

pub fn format_city(id: i32) -> anyhow::Result<String> {
    Ok(format!(
        "{} ФО, {}, {}",
        COUNTIES.get(&(id >> 16)).context("county not found")?,
        SUBJECTS
            .get(&((id >> 8) % 2i32.pow(8)))
            .context("subject not found")?,
        CITIES.get(&id).context("city not found")?,
    ))
}

pub fn county_by_id(id: i32) -> Option<&'static &'static str> {
    COUNTIES.get(&(id >> 16))
}

pub fn subject_by_id(id: i32) -> Option<&'static &'static str> {
    SUBJECTS.get(&((id >> 8) % 2i32.pow(8)))
}

pub fn city_by_id(id: i32) -> Option<&'static &'static str> {
    CITIES.get(&id)
}

pub fn county_exists(name: &str) -> bool {
    COUNTIES_REV.get(name).is_some()
}

pub fn subject_exists(name: &str) -> bool {
    SUBJECTS_REV.get(name).is_some()
}

pub fn city_exists(name: &str) -> bool {
    CITIES_REV.get(name).is_some()
}
