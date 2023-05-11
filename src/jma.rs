//! Precipitation Library
//! Using Japan Meteorological Agency API

use crate::utils::{PartOfDay, PointOfDay};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Local, Utc};
use image::{io::Reader, DynamicImage};
use once_cell::sync::Lazy;
use reqwest;
use serde::Deserialize;
use serde_json;
use std::io::Cursor;
use std::sync::Mutex;

// Tile Image Cache
const CACHE_SIZE: usize = 12;
static CACHE: Lazy<Mutex<Vec<Option<TileMeta>>>> = Lazy::new(|| {
    let v: Vec<Option<TileMeta>> = vec![None; CACHE_SIZE];
    Mutex::new(v)
});

fn cache_push(meta: TileMeta) {
    CACHE.lock().unwrap().remove(0);
    CACHE.lock().unwrap().push(Some(meta));
}

fn cache_search(meta: &TileMeta) -> Result<TileMeta, ()> {
    for c in CACHE.lock().unwrap().iter() {
        if let Some(m) = c.as_ref() {
            if m == meta {
                return Ok(m.clone());
            }
        }
    }
    Err(())
}

const API: &str = "https://www.jma.go.jp/bosai/forecast/data/forecast/";

#[derive(Debug, Clone)]
pub struct Forecast {
    /// Any
    pub area_name: String,
    pub area_code: AreaCode,
    /// Update datetime of morning and afternoon forecast.
    pub update: DateTime<Local>,
    /// Morning Probability of Precipitation [%] (from 6 a.m. to 12 a.m.)
    pub morning: Option<usize>,
    /// Afternoon Probability of Precipitation [%] (from 12 a.m. to 6 p.m.)
    pub afternoon: Option<usize>,
}

/// <https://www.jma.go.jp/bosai/common/const/area.json>
#[derive(Debug, Deserialize, Clone)]
pub struct AreaCode {
    pub offices: String,
    pub class10s: String,
    pub pops: Option<usize>,
    pub area_name: String,
    pub longitude: f64,
    pub latitude: f64,
    pub precipitation: f64,
}

impl Forecast {
    /// Creates a new Forecast instance
    pub fn new() -> Forecast {
        Forecast {
            area_name: String::new(),
            area_code: AreaCode {
                offices: String::new(),
                class10s: String::new(),
                pops: None,
                area_name: "Mount Fuji".to_string(),
                longitude: 35.362925,
                latitude: 138.731451,
                precipitation: 1.0,
            },
            update: Local::now(),
            morning: None,
            afternoon: None,
        }
    }

    /// Get weather forecast from JMA
    fn get_forecast(&self, pref: &str) -> String {
        let api_url = format!("{}{}.json", API, pref);
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap();
        let body = client.get(api_url).send().unwrap().text().unwrap();
        body
    }

    /// Calling get_forecast() and update 'morning' and 'afternoon' fields
    pub fn update(&mut self) {
        let text = self.get_forecast(&self.area_code.offices);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        let mut morning_index = None; // 06:00:00
        let mut afternoon_index = None; // 12:00:00
        let now = Local::now();
        self.update = now;
        let morning = PointOfDay::Dawn.datetime(now);
        let afternoon = PointOfDay::Noon.datetime(now);
        let time_list = json[0]["timeSeries"][1]["timeDefines"].as_array().unwrap();
        for (i, t) in time_list.iter().enumerate() {
            let t = DateTime::parse_from_rfc3339(t.as_str().unwrap()).unwrap();
            if morning == t {
                morning_index = Some(i);
            }
            if afternoon == t {
                afternoon_index = Some(i);
            }
        }

        let pops_list = json[0]["timeSeries"][1]["areas"].as_array().unwrap();
        let mut pops = None;
        for p in pops_list {
            if p["area"]["code"] == self.area_code.class10s {
                pops = Some(p);
                break;
            }
        }
        if let Some(pops) = pops {
            self.area_name = String::from(pops["area"]["name"].as_str().unwrap());
            if let Some(i) = morning_index {
                self.morning = Some(pops["pops"][i].as_str().unwrap().parse::<usize>().unwrap());
            }
            if let Some(i) = afternoon_index {
                self.afternoon = Some(pops["pops"][i].as_str().unwrap().parse::<usize>().unwrap());
            }
        }
    }
}

/// Tile Meta Data
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct TileMeta {
    basetime: String,
    validtime: String,
    member: String,
    elements: Vec<String>,
    #[serde(skip)]
    precipitation: Option<f32>,
    #[serde(skip)]
    image: String,
}

impl PartialEq for TileMeta {
    fn eq(&self, other: &Self) -> bool {
        self.basetime == other.basetime
            && self.validtime == other.validtime
            && self.member == other.member
    }
}

impl TileMeta {
    /// Creates a new TileMeta instance
    async fn new() -> Vec<TileMeta> {
        let catalog_str = TileMeta::get_catalog().await;
        let catalog: Vec<TileMeta> = serde_json::from_str(&catalog_str).unwrap();
        catalog
    }

    /// Get Tile Catalog from JMA
    async fn get_catalog() -> String {
        let api_url = "https://www.jma.go.jp/bosai/jmatile/data/rasrf/targetTimes.json";
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap();
        let body = client
            .get(api_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        body
    }

    fn validtime(&self) -> DateTime<Utc> {
        let validtime_utc_str = format!("{}{}", self.validtime, "+0000");
        DateTime::parse_from_str(&validtime_utc_str, "%Y%m%d%H%M%S%z")
            .unwrap()
            .into()
    }
}

#[derive(Debug, Clone)]
pub struct TileResult {
    pub precipitation: f32,
    pub images: Vec<String>,
    pub times: Vec<String>,
}

impl TileResult {
    pub fn new() -> TileResult {
        TileResult {
            precipitation: 0.0,
            images: Vec::<String>::new(),
            times: Vec::<String>::new(),
        }
    }
}

/// Area for Precipitation
///
/// This tile is a part of the GSI tile collection. (GSI: Geospatial Information Authority of Japan map)
///
/// The tile size is 256x256 pixel.
///
/// # Examples
/// ```
/// use sunnyday::jma::Tile;
/// use sunnyday::utils::PartOfDay;
///
/// async fn example() {
///     let mut tile = Tile::from_latlon(10, 35.685175, 193.7528);
///     let precipitation = tile.precipitation(PartOfDay::Morning).await.unwrap();
///     assert!(precipitation >= 0.0);
///}
/// ```
pub struct Tile {
    zoom: usize,
    x: usize,
    y: usize,
}

impl Tile {
    /// Create a new Tile instance with calculated zoom level from latitude and longitude
    pub fn from_latlon(zoom: usize, lat: f64, lon: f64) -> Tile {
        let base: f64 = 2.0;
        let n = base.powf(zoom as f64);
        let x: usize = ((lon + 180.0) / 360.0 * n) as usize;
        let lat_rad = lat.to_radians();
        let y: usize = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n) as usize;
        Tile { zoom, x, y }
    }

    /// Get PNG image of rain clouds from JMA (Japan Meteorogical Agency)
    ///
    /// This function use a JMA API.
    ///
    /// https://www.jma.go.jp/bosai/jmatile/data/rasrf/{basetime}/{member}/{validtime}/surf/rasrf/{z}/{x}/{y}.png
    /// 
    async fn get_tile(&self, meta: &TileMeta) -> Option<image::DynamicImage> {
        let url = format!(
            "https://www.jma.go.jp/bosai/jmatile/data/rasrf/{basetime}/{member}/{validtime}/surf/rasrf/{z}/{x}/{y}.png",
            basetime=meta.basetime,
            member=meta.member,
            validtime=meta.validtime,
            z=self.zoom,
            x=self.x,
            y=self.y,
        );
        let resp = reqwest::get(url).await.unwrap();
        let png_bytes = resp.bytes().await.unwrap();
        let reader = Reader::new(Cursor::new(png_bytes))
            .with_guessed_format()
            .unwrap();
        match reader.decode() {
            Ok(image) => Some(image),
            Err(why) => {
                println!("{:?}", meta);
                println!("{}", why.to_string());
                None
            }
        }
    }

    fn base64png(png: &DynamicImage) -> Option<String> {
        let small_png = png.resize(32, 32, image::imageops::FilterType::Nearest);
        let mut png_data = Vec::new();
        if small_png
            .write_to(
                &mut Cursor::new(&mut png_data),
                image::ImageOutputFormat::Png,
            )
            .is_ok()
        {
            return Some(general_purpose::STANDARD.encode(png_data));
        }
        None
    }

    async fn get_tiles(&mut self, metas: &mut Vec<TileMeta>) {
        for mut meta in metas {
            if let Ok(cache) = cache_search(meta) {
                meta.precipitation = cache.precipitation;
                meta.image = cache.image.to_string();
                if meta.precipitation != None {
                    continue;
                }
            }
            if let Some(tile_image) = self.get_tile(meta).await {
                meta.precipitation = Some(Tile::count_precipitation(&tile_image));
                meta.image = match Tile::base64png(&tile_image) {
                    Some(b) => b,
                    None => "".to_string(),
                };
            } else {
                meta.precipitation = None;
                meta.image = String::new();
            }
            cache_push(meta.clone());
        }
    }

    /// Count precipitation [mm/pixel] from PNG image
    fn count_precipitation(image: &image::DynamicImage) -> f32 {
        //let image = image::open("world.png").unwrap();
        let buffer = image.to_rgba8();
        let mut precipitation = 0;
        for x in 0..256 {
            for y in 0..256 {
                let rgba = buffer.get_pixel(x, y);
                let intensity = match rgba {
                    image::Rgba([180, 0, 104, 255]) => (8, 100), // Violet
                    image::Rgba([255, 40, 0, 255]) => (7, 80),   // Red
                    image::Rgba([255, 153, 0, 255]) => (6, 50),  // Orange
                    //image::Rgba([255, 245, 0, 255]) => (5, 30),  // Orange
                    image::Rgba([250, 245, 0, 255]) => (5, 30), // Yellow
                    image::Rgba([0, 65, 255, 255]) => (4, 20),  // Blue
                    image::Rgba([33, 140, 255, 255]) => (3, 10), // Water
                    image::Rgba([160, 210, 255, 255]) => (2, 5), // Sky Blue
                    image::Rgba([242, 242, 255, 255]) => (1, 1), // Subtle blue
                    image::Rgba([0, 0, 0, 0]) => (0, 0),        // Clear
                    image::Rgba([255, 255, 255, 0]) => (0, 0),  // Clear
                    _ => {
                        panic!("({},{}) = {:?}", x, y, rgba);
                    }
                };
                precipitation += intensity.1;
            }
        }
        precipitation as f32 / (256.0 * 256.0)
    }

    #[allow(dead_code)]
    pub async fn precipitation(&mut self, part: PartOfDay) -> Result<f32, String> {
        match self.precipitation_with_images(part).await {
            Ok(r) => Ok(r.precipitation),
            Err(why) => Err(why),
        }
    }

    pub async fn precipitation_with_images(
        &mut self,
        part: PartOfDay,
    ) -> Result<TileResult, String> {
        // check datetime
        let now_jst = Local::now();
        let mut begin: DateTime<Utc> = part.begin().datetime(now_jst).into();
        let end: DateTime<Utc> = part.end().datetime(now_jst).into();

        let now = Utc::now();
        if now > end {
            return Err(format!("Out of {:?}", part));
        }
        if now > begin {
            begin = now;
        }

        debug_assert!(begin >= now);
        debug_assert!(end > now);

        let catalog = TileMeta::new().await;
        let mut now_index = 0;
        let mut duration_min = Duration::days(365).num_seconds();
        for (i, m) in catalog.iter().enumerate() {
            let validtime = m.validtime();
            let diff = (validtime - begin).num_seconds().abs();
            if diff < duration_min {
                duration_min = diff;
                now_index = i;
            }
        }

        debug_assert!(
            {
                let cat = catalog[now_index].validtime();
                let previous = if now_index == 0 { 0 } else { now_index - 1 };
                let cat_p = catalog[previous].validtime();
                let next = if now_index + 1 >= catalog.len() {
                    now_index
                } else {
                    now_index + 1
                };
                let cat_n = catalog[next].validtime();
                (cat - begin).num_seconds().abs() <= (cat_p - begin).num_seconds().abs()
                    && (cat - begin).num_seconds().abs() <= (cat_n - begin).num_seconds().abs()
            },
            "\nbegin: {}, end: {}\nnow: {}\n  previous: {}\n  selected: {}\n      next: {}\n",
            begin,
            end,
            now,
            catalog[if now_index == 0 { 0 } else { now_index - 1 }].validtime,
            catalog[now_index].validtime,
            catalog[if now_index + 1 >= catalog.len() {
                now_index
            } else {
                now_index + 1
            }]
            .validtime
        );

        // get images of rain cloud and calculate precipitation
        let mut precipitation_max: Result<f32, String> = Err("".to_string());
        let mut res = TileResult::new();
        let mut metas: Vec<TileMeta> = Vec::new();
        for i in (0..=now_index).rev() {
            if catalog[i].validtime() > end {
                break;
            }
            metas.push(catalog[i].clone());
        }
        self.get_tiles(&mut metas).await;
        for meta in metas {
            precipitation_max = match precipitation_max {
                Ok(p) => match meta.precipitation {
                    Some(mp) => {
                        if p < mp {
                            Ok(mp)
                        } else {
                            Ok(p)
                        }
                    }
                    None => Ok(p),
                },
                Err(_) => match meta.precipitation {
                    Some(mp) => Ok(mp),
                    None => Err("".to_string()),
                },
            };
            res.times.push(
                meta.validtime()
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string(),
            );
            res.images.push(meta.image);
        }
        match precipitation_max {
            Ok(p) => res.precipitation = p,
            Err(_) => return Err("No Precipitation data".to_string()),
        };
        Ok(res)
    }
}

#[tokio::test]
async fn precipitation_test() {
    let mut t = Tile {
        zoom: 10,
        x: 910,
        y: 403,
    };
    let p = t.precipitation(PartOfDay::Afternoon).await.unwrap();
    println!("Maximum Precipitation: {} mm/h", p);
    assert!(p >= 0.0);
}

#[test]
fn tile_test() {
    let t = Tile::from_latlon(10, 35.681240, 139.752766);
    assert_eq!(t.zoom, 10);
    assert_eq!(t.x, 909);
    assert_eq!(t.y, 403);

    let t = Tile::from_latlon(10, 43.0686663, 141.3507557);
    assert_eq!(t.zoom, 10);
    assert_eq!(t.x, 914);
    assert_eq!(t.y, 376);

    let t = Tile::from_latlon(12, 24.3904605, 124.2460321);
    assert_eq!(t.zoom, 12);
    assert_eq!(t.x, 3461);
    assert_eq!(t.y, 1761);

    let t = Tile::from_latlon(10, 26.8658607, 128.2530679);
    assert_eq!(t.zoom, 10);
    assert_eq!(t.x, 876);
    assert_eq!(t.y, 432);
}

#[test]
fn count_precipitation_test() {
    let pattern = [
        ("share/00mm.png", 0.0),
        ("share/01mm.png", 0.535141),
        ("share/30mm.png", 20.270538),
        ("share/80mm.png", 29.347229),
    ];
    for pat in pattern {
        let pre = Tile::count_precipitation(&image::open(pat.0).unwrap());
        println!("{}: {}", pat.0, pat.1);
        assert_eq!(pre, pat.1);
    }
}

#[test]
fn get_forecast_test() {
    let f = Forecast::new();
    assert!(f.get_forecast("020000").len() > 0);
}
