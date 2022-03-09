use std::{panic, process::exit};

use clap::StructOpt;
use env_logger::Env;
use error::CliError;
use log::{error, info};
use utils::user::get_creds_and_config;

pub mod commands;
mod config;
pub mod error;
mod utils;

#[derive(Debug, StructOpt)]
#[structopt(about = "CLI for Momento APIs")]
struct Momento {
    #[structopt(name = "verbose", global = true, long)]
    verbose: bool,

    #[structopt(subcommand)]
    command: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    #[structopt(about = "Cache Operations")]
    Cache {
        #[structopt(subcommand)]
        operation: CacheCommand,
    },
    #[structopt(about = "Configure Momento Credentials")]
    Configure {
        #[structopt(name = "profile", long, short, default_value = "default")]
        profile: String,
    },
    #[structopt(about = "Manage Accounts")]
    Account {
        #[structopt(subcommand)]
        operation: AccountCommand,
    },
}

#[derive(Debug, StructOpt)]
enum AccountCommand {
    #[structopt(about = "Sign up for Momento")]
    Signup {
        #[structopt(name = "email", long, short)]
        email: String,
        #[structopt(
            name = "region",
            long,
            short,
            default_value = "us-west-2",
            help = "e.g. us-west-2, us-east-1, ap-northeast-1"
        )]
        region: String,
    },
}

#[derive(Debug, StructOpt)]
enum CacheCommand {
    #[structopt(about = "Creates a Momento Cache")]
    Create {
        #[structopt(name = "name", long, short)]
        cache_name: String,
        #[structopt(name = "profile", long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Stores a given item in cache")]
    Set {
        #[structopt(name = "name", long, short)]
        cache_name: Option<String>,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,
        #[structopt(long, short)]
        value: String,
        #[structopt(
            long = "ttl",
            short = 't',
            help = "Max time, in seconds, that the item will be stored in cache"
        )]
        ttl_seconds: Option<u32>,
        #[structopt(name = "profile", long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Gets item from the cache")]
    Get {
        #[structopt(name = "name", long, short)]
        cache_name: Option<String>,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,
        #[structopt(name = "profile", long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Deletes the cache")]
    Delete {
        #[structopt(name = "name", long, short)]
        cache_name: String,
        #[structopt(name = "profile", long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Lists all momento caches")]
    List {
        #[structopt(name = "profile", long, short, default_value = "default")]
        profile: String,
    },
}

async fn entrypoint() -> Result<(), CliError> {
    let args = Momento::parse();

    let log_level = if args.verbose { "debug" } else { "info" };

    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or(log_level)
            .default_write_style_or("always"),
    )
    .init();

    match args.command {
        Subcommand::Cache { operation } => match operation {
            CacheCommand::Create {
                cache_name,
                profile,
            } => {
                let (creds, _config) = get_creds_and_config(&profile).await?;
                commands::cache::cache_cli::create_cache(cache_name.clone(), creds.token).await?;
                info!("created cache {cache_name}")
            }
            CacheCommand::Set {
                cache_name,
                key,
                value,
                ttl_seconds,
                profile,
            } => {
                let (creds, config) = get_creds_and_config(&profile).await?;
                commands::cache::cache_cli::set(
                    cache_name.unwrap_or(config.cache),
                    creds.token,
                    key,
                    value,
                    ttl_seconds.unwrap_or(config.ttl),
                )
                .await?
            }
            CacheCommand::Get {
                cache_name,
                key,
                profile,
            } => {
                let (creds, config) = get_creds_and_config(&profile).await?;
                commands::cache::cache_cli::get(
                    cache_name.unwrap_or(config.cache),
                    creds.token,
                    key,
                )
                .await?;
            }
            CacheCommand::Delete {
                cache_name,
                profile,
            } => {
                let (creds, _config) = get_creds_and_config(&profile).await?;
                commands::cache::cache_cli::delete_cache(cache_name.clone(), creds.token).await?;
                info!("deleted cache {}", cache_name)
            }
            CacheCommand::List { profile } => {
                let (creds, _config) = get_creds_and_config(&profile).await?;
                commands::cache::cache_cli::list_caches(creds.token).await?
            }
        },
        Subcommand::Configure { profile } => {
            commands::configure::configure_cli::configure_momento(&profile).await?
        }
        Subcommand::Account { operation } => match operation {
            AccountCommand::Signup { email, region } => {
                commands::account::signup_user(email, region).await?;
            }
        },
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    panic::set_hook(Box::new(move |info| {
        error!("{}", info);
    }));

    if let Err(e) = entrypoint().await {
        eprintln!("{}", e);
        exit(1)
    }
}
