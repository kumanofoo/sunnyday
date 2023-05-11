use axum::{routing::get, Router};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use sunnyday::home;
use sunnyday::place::Places;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short = 'p', long, help = "TCP port")]
    pub port: Option<u16>,
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

const DEFAULT_PORT: u16 = 3000;

pub async fn webserver(cli: &Cli, places: Places) {
    let port: u16 = cli.port.unwrap_or(DEFAULT_PORT);
    let app = Router::new()
        .route("/", get(home::place_handler))
        .with_state(places);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    cli.verbose(format!("Listening on {}", port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let places: Places = match Places::read(&PathBuf::from("place.toml")) {
        Ok(r) => r,
        Err(why) => {
            println!("{}", why.to_string());
            std::process::exit(1);
        }
    };

    webserver(&cli, places).await;
}
