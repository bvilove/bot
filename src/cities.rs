use std::cell::LazyCell;

use simsearch::SimSearch;

pub const CITIES: LazyCell<Vec<Vec<&str>>> = LazyCell::new(|| {
    vec![
        vec!["Москва"],
        vec!["Питербург", "Санкт-Питербург", "Ленинград"],
        vec!["Мордовия", "Саранск"],
    ]
});

const ENGINE: LazyCell<SimSearch<usize>> = LazyCell::new(|| {
    let mut engine: SimSearch<usize> = SimSearch::new();
    for (city_id, city) in CITIES.iter().enumerate() {
        for (alias_id, city_alias) in city.iter().enumerate() {
            engine.insert(city_id * 100 + alias_id, city_alias);
        }
    }
    engine
});

pub fn find_city(query: &str) -> Option<(usize, &str)> {
    let results = ENGINE.search(query);
    results.first().map(|r| (r / 100, CITIES[r / 100][0]))
}
