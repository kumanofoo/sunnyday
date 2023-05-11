//! Today's mood

use crate::jma::{self, TileResult};
use crate::utils::PartOfDay;

/// What are you in the mood for?
#[derive(Debug, Clone)]
pub struct Mood {
    pub food: Option<bool>,
    pub walking: Option<bool>,
    pub parking: Option<bool>,
    pub part_of_day: Option<PartOfDay>,
    pub forecast: Option<TileResult>,
}

impl Mood {
    pub fn new() -> Mood {
        Mood {
            food: None,
            walking: None,
            parking: None,
            part_of_day: None,
            forecast: None,
        }
    }

    pub fn set_part_of_day(&mut self, part: PartOfDay) {
        self.part_of_day = Some(part);
    }

    pub fn unset_part_of_day(&mut self) {
        self.part_of_day = None;
    }

    /// Get precipitation and dicide if do walking
    pub async fn check_precipitation(&mut self, area_code: &jma::AreaCode) -> Option<f32> {
        let mut prec: Option<f32> = None;
        match self.part_of_day {
            Some(part) => {
                let mut tile = jma::Tile::from_latlon(10, area_code.latitude, area_code.longitude);
                self.walking = match tile.precipitation_with_images(part).await {
                    Ok(r) => {
                        let p = r.precipitation;
                        prec = Some(p);
                        self.forecast = Some(r);
                        if (p as f64) > area_code.precipitation {
                            Some(false)
                        } else {
                            Some(true)
                        }
                    }
                    Err(_) => None,
                };
            }
            None => (),
        }
        prec
    }

    /// Get probability of precipitation and dicide if do walking
    pub fn check_probability(&mut self, area_code: &jma::AreaCode) -> Option<usize> {
        let mut pop: Option<usize> = None;

        match self.part_of_day {
            Some(part) => {
                let mut forecast = jma::Forecast::new();
                forecast.area_code = area_code.clone();
                forecast.update();
                let p = match part {
                    PartOfDay::Morning => {
                        pop = forecast.morning;
                        forecast.morning
                    }
                    PartOfDay::Afternoon => {
                        pop = forecast.afternoon;
                        forecast.afternoon
                    }
                };
                self.walking = match p {
                    Some(p) => {
                        let limit = area_code.pops.unwrap_or(10);
                        if p > limit {
                            Some(false)
                        } else {
                            Some(true)
                        }
                    }
                    None => None,
                };
            }
            None => (),
        }
        pop
    }

    /// Create String from each parameter of mood
    pub fn to_string(&self) -> String {
        let food = match self.food {
            Some(p) => {
                if p {
                    "yes"
                } else {
                    "no"
                }
            }
            None => "-",
        };

        let walking = match self.walking {
            Some(p) => {
                if p {
                    "yes"
                } else {
                    "no"
                }
            }
            None => "-",
        };

        let parking = match self.parking {
            Some(p) => {
                if p {
                    "yes"
                } else {
                    "no"
                }
            }
            None => "-",
        };

        format!("Food: {}, Walking: {}, Parking: {}", food, walking, parking,)
    }
}
