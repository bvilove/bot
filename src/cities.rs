use std::sync::OnceLock;

use simsearch::SimSearch;

macro_rules! lazy_cell {
    ($name:tt, $t:ty, $init:expr) => {
        #[allow(non_snake_case)]
        pub fn $name() -> &'static $t {
            static LOCK: OnceLock<$t> = OnceLock::new();
            LOCK.get_or_init($init)
        }
    };
}

lazy_cell!(CITIES, Vec<Vec<&'static str>>, || {
    vec![
        vec!["Москва"],
        vec!["Питербург", "Санкт-Питербург", "Ленинград"],
        vec!["Мордовия", "Саранск"],
    ]
});

lazy_cell!(ENGINE, SimSearch<usize>, || {
    let mut engine: SimSearch<usize> = SimSearch::new();
    for (city_id, city) in CITIES().iter().enumerate() {
        for (alias_id, city_alias) in city.iter().enumerate() {
            engine.insert(city_id * 100 + alias_id, city_alias);
        }
    }
    engine
});

pub fn find_city(query: &str) -> Option<(usize, &str)> {
    let results = ENGINE().search(query);
    results.first().map(|r| (r / 100, CITIES()[r / 100][0]))
}
