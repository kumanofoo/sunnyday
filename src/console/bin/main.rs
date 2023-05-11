use clap::{ArgGroup, Parser};
use rand::prelude::SliceRandom;
use std::path::PathBuf;
use std::process::exit;
use sunnyday::mood::Mood;
use sunnyday::place::{Places, RecentPlace};
use sunnyday::utils::{PartOfDay, ALL_DAY};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None,
          group(ArgGroup::new("how_food").required(false).args(["food", "no_food"])),
          group(ArgGroup::new("how_walking").required(false).args(["walking", "no_walking"])),
          group(ArgGroup::new("how_parking").required(false).args(["parking", "no_parking"])))]
pub struct Cli {
    #[arg(short = 'f', long, help = "with food")]
    pub food: bool,
    #[arg(short = 'F', long, help = "without food")]
    pub no_food: bool,
    #[arg(short = 'w', long, help = "with walking")]
    pub walking: bool,
    #[arg(short = 'W', long, help = "without walking")]
    pub no_walking: bool,
    #[arg(short = 'p', long, help = "with parking")]
    pub parking: bool,
    #[arg(short = 'P', long, help = "without parking")]
    pub no_parking: bool,
    #[arg(long, help = "use probability")]
    pub use_probability: bool,
    #[arg(short = 'v', long, help = "verbose mode")]
    pub verbose: bool,
    #[arg(short = 'r', long, help = "display recent places")]
    pub recent: bool,
    #[arg(long, default_value = ".place_recent", help = "recent places file")]
    pub recent_file: String,
}

impl Cli {
    fn verbose(&self, message: impl AsRef<str>) {
        if self.verbose {
            println!("{}", message.as_ref());
        }
    }
}

fn get_mood(cli: &Cli) -> Mood {
    // walking
    //                     option
    //                | none  true false
    //          ------+-----------------
    //          none  | none  true false
    // forecast true  | true  true false
    //          false | false true false
    //
    let mut food: Option<bool> = None;
    let mut walking: Option<bool> = None;
    let mut parking: Option<bool> = None;

    // Check food options
    if cli.food {
        food = Some(true);
        if cli.verbose {
            println!("With food");
        }
    }
    if cli.no_food {
        food = Some(false);
        if cli.verbose {
            println!("Without food");
        }
    }

    // Check walking options
    if cli.walking {
        walking = Some(true);
        if cli.verbose {
            println!("With walking");
        }
    }
    if cli.no_walking {
        walking = Some(false);
        if cli.verbose {
            println!("Without walking");
        }
    }

    // Check parking options
    if cli.parking {
        parking = Some(true);
        if cli.verbose {
            println!("With parking");
        }
    }
    if cli.no_parking {
        parking = Some(false);
        if cli.verbose {
            println!("Without parking");
        }
    }
    Mood {
        food,
        walking,
        parking,
        part_of_day: None,
        forecast: None,
    }
}

/// Get today's place with mood
pub async fn today_place(cli: &Cli, places: Places, mut recent: RecentPlace) {
    let mood_now = get_mood(cli);
    let mut moods = Vec::<Mood>::new();

    // don't change walking mood if it is already set
    if mood_now.walking == None {
        // check weather forecast
        if let Some(area_code) = places.area_code.clone() {
            cli.verbose(format!(
                "{} ({}, {})",
                &area_code.area_name, area_code.latitude, area_code.longitude
            ));
            for part in ALL_DAY {
                let mut m = mood_now.clone();
                m.part_of_day = Some(part);
                if cli.use_probability {
                    if let Some(p) = m.check_probability(&area_code) {
                        cli.verbose(format!("  {}: {}%", part.to_string(), p));
                    } else {
                        cli.verbose(format!("  {}: No probability", part.to_string()));
                    }
                } else {
                    if let Some(p) = m.check_precipitation(&area_code).await {
                        cli.verbose(format!("  {}: {:.1}mm/h", part.to_string(), p));
                    } else {
                        cli.verbose(format!("  {}: No precipitation", part.to_string()));
                    }
                }
                moods.push(m);
            }
        } else {
            // no area code
            moods.push(mood_now);
        }
    } else {
        // specified walking mood by command line option
        // so don't care weather infomation
        moods.push(mood_now);
    }

    // pickup places
    for m in moods {
        let mut available = places.pickup(&m);
        let mut rng = rand::thread_rng();
        available.shuffle(&mut rng);

        if let Some(part) = m.part_of_day {
            println!("{}", part.to_string());
            cli.verbose(m.to_string());
            let mut today_place: Option<String> = None;
            for p in available {
                if recent.check(&p.name, part) {
                    continue;
                }
                today_place = Some(p.name);
                break;
            }
            match today_place {
                Some(p) => {
                    recent.today_place(&p, part);
                    recent.save().unwrap();
                    println!("  {}", p);
                }
                None => println!("  no place is recommended."),
            }
        } else {
            println!("today");
            println!("  {}", available[0].name);
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // read configration
    let places: Places = match Places::read(&PathBuf::from("place.toml")) {
        Ok(p) => p,
        Err(why) => {
            println!("{}", why.to_string());
            exit(1);
        }
    };

    // read recent place
    let recent_places = match RecentPlace::read(&PathBuf::from(&cli.recent_file)) {
        Ok(r) => r,
        Err(why) => {
            println!("{:?}", why);
            RecentPlace::new()
        }
    };
    if cli.recent {
        println!("[Recent Place]");
        println!("Morning");
        for r in recent_places.get_places(PartOfDay::Morning) {
            println!("  {}", r);
        }
        println!("Afternoon");
        for r in recent_places.get_places(PartOfDay::Afternoon) {
            println!("  {}", r);
        }
        return;
    }

    today_place(&cli, places, recent_places).await;
}
