use crate::configuration::Configuration;
use clap::Parser;
use dotenvy::dotenv;
use tracing::info;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short = 't', long = "title", 
        help = "Website Title")]
    website_title: Option<String>,

    #[arg(short = 'k', long = "key", 
        help = "Authentication key for API access")]
    password: Option<String>,

    #[arg(short = 'p', long = "port", 
        help = "Port number for the HTTP server")]
    port: Option<String>,

    #[arg(
        short = 'd', 
        long = "database", 
        default_missing_value = "", 
        num_args = 0..=1, 
        help = "Database connection. Without this argument the timeslots are not stored persistently",
    )]
    database_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ConfigurationHandler {
    website_title: String,
    password: String,
    frontend_path: PathBuf,
    database_url: Option<String>,
    port: String,
}

impl ConfigurationHandler {
    pub fn parse_arguments() -> Self {
        let args = Cli::parse();

        dotenv().expect("Failed to load .env file");
        let website_title = if let Some(website_title) = args.website_title {
            info!("Website Title provided as argument");
            website_title
        } else {
            info!("Website Title not provided as argument. Using WEBSITE_TITLE specified in \".env\".");
            env::var("WEBSITE_TITLE").expect("WEBSITE_TITLE must be set in .env file")
        };

        let password = if let Some(password) = args.password {
            info!("Password provided as argument");
            password
        } else {
            info!("Password not provided as argument. Using HTTP_PASSWORD specified in \".env\".");
            env::var("HTTP_PASSWORD").expect("HTTP_PASSWORD must be set in .env file")
        };

        let port = if let Some(port) = args.port {
            info!("Port provided as argument");
            port
        } else {
            info!("No port provided as argument. Using PORT specified in \".env\" file");
            env::var("PORT").expect("PORT must be set in .env file")
        };

        let database_url = if let Some(database_url) = args.database_url {
            if database_url.is_empty() {
                info!("Run with database. No database url provided as argument. Using DATABASE_URL specified in \".env\" file");
                Some(env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file"))
            } else {
                info!("Run with database. Database url provided as argument");
                Some(database_url)
            }
        } else {
            info!("Run without database");
            None
        };

        Self {
            website_title,
            password,
            frontend_path: PathBuf::from("../frontend/index.html"),
            database_url,
            port,
        }
    }
}

impl Configuration for ConfigurationHandler {
    fn website_title(&self) -> String {
        self.website_title.clone()
    }

    fn password(&self) -> String {
        self.password.clone()
    }

    fn frontend_path(&self) -> PathBuf {
        self.frontend_path.clone()
    }

    fn database_url(&self) -> Option<String> {
        self.database_url.clone()
    }

    fn port(&self) -> String {
        self.port.clone()
    }
}
