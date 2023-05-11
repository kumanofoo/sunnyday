//! Favorite Place

use crate::jma;
use crate::mood::Mood;
use crate::utils::PartOfDay;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;

/// Place information
#[derive(Debug, Deserialize, Clone)]
pub struct Place {
    pub name: String,
    pub shop: Vec<String>,
    pub walking: bool,
    pub parking: bool,
}

/// Shop information
#[derive(Debug, Deserialize, Clone)]
pub struct Shop {
    pub name: String,
    pub food: bool,
}

/// Place database
#[derive(Debug, Deserialize, Clone)]
pub struct Places {
    pub area_code: Option<jma::AreaCode>,
    pub parking: Vec<Place>,
    pub shop: Vec<Shop>,
}

impl Places {
    /// Read Place DB from TOML file
    pub fn read(filename: &PathBuf) -> Result<Places, String> {
        let config_file: String = match fs::read_to_string(filename) {
            Ok(c) => c,
            Err(why) => return Err(why.to_string()),
        };
        let config: Places = match toml::de::from_str(&config_file) {
            Ok(p) => p,
            Err(why) => return Err(why.to_string()),
        };
        Ok(config)
    }

    /// Pickup places considering mood
    pub fn pickup(&self, mood: &Mood) -> Vec<Place> {
        let mut places: Vec<Place> = Vec::new();
        for p in &self.parking {
            let food = match mood.food {
                Some(f) => {
                    let mut res = false;
                    for s in &p.shop {
                        if self.shop.iter().any(|x| x.name == *s && x.food == f) {
                            res = true;
                            break;
                        }
                    }
                    res
                }
                None => true,
            };
            let walking = match mood.walking {
                Some(b) => p.walking == b,
                None => true,
            };
            let parking = match mood.parking {
                Some(b) => p.parking == b,
                None => true,
            };

            if food && walking && parking {
                places.push(p.clone());
            }
        }
        places
    }
}

/// History of suggested Places
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecentPlace {
    rotation_days: Option<usize>,
    morning: Vec<String>,
    afternoon: Vec<String>,
    #[serde(skip)]
    filename: PathBuf,
}

const DEFAULT_ROTATION_DAYS: usize = 7;
const DEFAULT_RECENT_PLACE_FILE: &str = ".place_recent";

impl RecentPlace {
    /// Create a new RecentPlace instance
    pub fn new() -> RecentPlace {
        RecentPlace {
            rotation_days: Some(DEFAULT_ROTATION_DAYS),
            morning: Vec::new(),
            afternoon: Vec::new(),
            filename: PathBuf::from(DEFAULT_RECENT_PLACE_FILE),
        }
    }

    /// Read suggested places from TOML file
    ///
    /// ```TOML
    /// rotation_days = 7
    /// morning = ['starbucks', 'McDonalds', 'Mt Fuji']
    /// afternoon = ['A&W', ''SUBWAY]
    /// ```
    pub fn read(filename: &PathBuf) -> Result<RecentPlace, String> {
        if !filename.exists() {
            return Ok(RecentPlace::new());
        }
        let recent_place_str = match fs::read_to_string(filename) {
            Ok(s) => s,
            Err(why) => return Err(why.to_string()),
        };
        let mut places: RecentPlace = match toml::de::from_str(&recent_place_str) {
            Ok(p) => p,
            Err(why) => return Err(why.to_string()),
        };
        if places.rotation_days == None {
            places.rotation_days = Some(DEFAULT_ROTATION_DAYS);
        }
        places.filename = filename.to_path_buf();
        Ok(places)
    }

    /// Save suggested places to TOML file
    ///
    /// ```TOML
    /// rotation_days = 7
    /// morning = ['starbucks', 'McDonalds', 'Mt Fuji']
    /// afternoon = ['A&W', ''SUBWAY]
    /// ```
    pub fn save(&self) -> Result<(), String> {
        let mut file = match File::create(&self.filename) {
            Ok(f) => f,
            Err(why) => return Err(why.to_string()),
        };
        let places = match toml::to_string(&self) {
            Ok(s) => s,
            Err(why) => return Err(why.to_string()),
        };
        if let Err(why) = file.write_all(places.as_bytes()) {
            return Err(why.to_string());
        }

        Ok(())
    }

    /// Check if include the place
    pub fn check(&mut self, place: &str, part: PartOfDay) -> bool {
        let p = match part {
            PartOfDay::Morning => &self.morning,
            PartOfDay::Afternoon => &self.afternoon,
        };

        p.iter().any(|x| x == place)
    }

    /// Add a place to DB
    pub fn today_place(&mut self, place: &str, part: PartOfDay) {
        let p = match part {
            PartOfDay::Morning => &mut self.morning,
            PartOfDay::Afternoon => &mut self.afternoon,
        };
        p.push(place.to_string());
        match self.rotation_days {
            Some(d) => {
                if p.len() >= d {
                    p.remove(0);
                }
            }
            None => (),
        }
    }

    /// Get places in part of day
    pub fn get_places(&self, part: PartOfDay) -> Vec<String> {
        match part {
            PartOfDay::Morning => self.morning.clone(),
            PartOfDay::Afternoon => self.afternoon.clone(),
        }
    }
}

#[test]
fn pick_test() {
    let places: Places = Places::read(&PathBuf::from("place.toml")).unwrap();
    let test_param = vec![Some(true), Some(false), None];
    for food in test_param.clone() {
        for walking in test_param.clone() {
            for parking in test_param.clone() {
                let mood = Mood {
                    food,
                    walking,
                    parking,
                    part_of_day: None,
                    forecast: None,
                };
                let available = places.pickup(&mood);
                for a in available {
                    match walking {
                        Some(w) => assert_eq!(a.walking, w),
                        None => (),
                    }
                    match parking {
                        Some(p) => assert_eq!(a.parking, p),
                        None => (),
                    }
                    match food {
                        Some(f) => {
                            let mut res = false;
                            for s in a.shop {
                                if places.shop.iter().any(|x| x.name == s && x.food == f) {
                                    res = true;
                                    break;
                                }
                            }
                            assert!(res);
                        }
                        None => (),
                    }
                }
            }
        }
    }
}

#[test]
fn read_place_test() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let file = dir.path().join("recent.place");
    let s = RecentPlace {
        rotation_days: Some(DEFAULT_ROTATION_DAYS),
        morning: vec!["alpha".to_string(), "bravo".to_string()],
        afternoon: vec!["charlie".to_string(), "delta".to_string()],
        filename: file,
    };
    match s.save() {
        Ok(_) => assert!(true),
        Err(why) => assert!(false, "{}", why),
    }

    let file = dir.path().join("recent.place");
    match RecentPlace::read(&file) {
        Ok(r) => {
            assert!(r.morning.len() == s.morning.len());
            for i in 0..r.morning.len() {
                assert!(r.morning[i] == s.morning[i]);
            }
            assert!(r.afternoon.len() == s.afternoon.len());
            for i in 0..r.afternoon.len() {
                assert!(r.afternoon[i] == s.afternoon[i]);
            }
            assert!(r.rotation_days == Some(DEFAULT_ROTATION_DAYS));
        }
        Err(why) => assert!(false, "{}", why),
    }
}
