use crate::mood::Mood;
use crate::place::Places;
use crate::utils::{PartOfDay, ALL_DAY};
use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use rand::prelude::SliceRandom;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "home.html")]
struct PlacesTemplate {
    morning_place: String,
    morning_images: Vec<(String, String)>,
    afternoon_place: String,
    afternoon_images: Vec<(String, String)>,
    weather_icon: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetMood {
    food: Option<bool>,
    parking: Option<bool>,
    walking: Option<bool>,
    weather: Option<bool>,
}

impl From<GetMood> for Mood {
    fn from(mood: GetMood) -> Mood {
        Mood {
            food: mood.food,
            parking: mood.parking,
            walking: mood.walking,
            part_of_day: None,
            forecast: None,
        }
    }
}

pub async fn place_handler(
    Query(get_param): Query<GetMood>,
    State(places): State<Places>,
) -> Html<String> {
    let mut morning_place = String::new();
    let mut morning_images: Vec<(String, String)> = Vec::new();
    let mut afternoon_place = String::new();
    let mut afternoon_images: Vec<(String, String)> = Vec::new();
    let mut wicon: String = String::new();

    for part in ALL_DAY {
        let mut mood = Mood::from(get_param.clone());
        mood.set_part_of_day(part);

        // get precipitation and set mood for walking
        if Some(false) != get_param.weather {
            if let Some(area_code) = places.area_code.clone() {
                if let Some(p) = mood.check_precipitation(&area_code).await {
                    if (p as f64) > area_code.precipitation {
                        wicon += "☂";
                    } else {
                        wicon += "☀";
                    }
                } else {
                }
            }
        }

        // pickup places depending mood
        let mut available = places.pickup(&mood);
        available.shuffle(&mut rand::thread_rng());
        let place: String = match available.pop() {
            Some(p) => p.name,
            None => "No Places".to_string(),
        };
        let mut images: Vec<(String, String)> = Vec::new();
        if let Some(f) = mood.forecast {
            for (t, i) in f.times.iter().zip(f.images) {
                images.push((t.to_string(), i));
            }
        }
        match part {
            PartOfDay::Morning => {
                morning_place = place;
                morning_images = images;
            }
            PartOfDay::Afternoon => {
                afternoon_place = place;
                afternoon_images = images;
            }
        }
    }

    let weather_icon = match wicon.as_str() {
        "☂☂" => "☂".to_string(),
        "☀☀" => "☀".to_string(),
        "" => "☀".to_string(),
        w => w.to_string(),
    };

    let place_template = PlacesTemplate {
        morning_place,
        morning_images,
        afternoon_place,
        afternoon_images,
        weather_icon,
    };
    Html(place_template.render().unwrap())
}
