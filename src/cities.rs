use std::{fmt::Display, str::FromStr};

use anyhow::Context;
use itertools::Itertools;
use strsim::jaro_winkler;

include!(concat!(env!("OUT_DIR"), "/citiesmap.rs"));

#[derive(Copy, Clone)]
pub struct City(Option<i32>);

impl Display for City {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(id) => {
                let county =
                    county_by_id(id).context("county not found").unwrap();
                let subject =
                    subject_by_id(id).context("subject not found").unwrap();
                let city = city_by_id(id).context("city not found").unwrap();

                if subject == city {
                    f.write_fmt(format_args!("{county} ФО, {city}"))?;
                } else {
                    f.write_fmt(format_args!(
                        "{county} ФО, {subject}, {city}"
                    ))?;
                }
            }
            None => f.write_str("Город не указан")?,
        }

        Ok(())
    }
}

impl FromStr for City {
    type Err = ();

    fn from_str(query: &str) -> Result<Self, Self::Err> {
        let best_city = CITIES
            .entries()
            .sorted_unstable_by(|(_, left), (_, right)| {
                jaro_winkler(&query.to_lowercase(), &left.to_lowercase())
                    .total_cmp(&jaro_winkler(
                        &query.to_lowercase(),
                        &right.to_lowercase(),
                    ))
            })
            .next_back()
            .expect("there must be at least 1 city");
        if jaro_winkler(best_city.1, query) > 0.15 {
            Ok(Self(Some(*best_city.0)))
        } else {
            Err(())
        }
    }
}

impl From<Option<i32>> for City {
    fn from(value: Option<i32>) -> Self {
        Self(value)
    }
}

impl From<City> for Option<i32> {
    fn from(value: City) -> Self {
        value.0
    }
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
