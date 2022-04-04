use std::{panic, process::exit};

use clap::StructOpt;
use env_logger::Env;
use error::CliError;
use log::{debug, error};
use utils::user::get_creds_and_config;

pub mod commands;
mod config;
pub mod error;
mod utils;

#[derive(Debug, StructOpt)]
#[clap(version)]
#[structopt(about = "CLI for Momento APIs", name = "momento")]
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
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },
    #[structopt(about = "Account Managements")]
    Account {
        #[structopt(subcommand)]
        operation: AccountCommand,
    },
}

#[derive(Debug, StructOpt)]
enum AccountCommand {
    #[structopt(about = "Sign up for Momento")]
    Signup {
        #[structopt(long, short)]
        email: String,
        #[structopt(
            long,
            short,
            default_value = "us-west-2",
            help = "e.g. us-west-2, us-east-1, ap-northeast-1"
        )]
        region: String,
    },

    #[structopt(about = "Creates a Momento Signing Key")]
    CreateSigningKey {
        #[structopt(
            long = "ttl",
            short = 't',
            default_value = "86400",
            help = "Duration, in minutes, that the signing key will be valid"
        )]
        ttl_minutes: u32,
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Revokes the given Momento Signing Key")]
    RevokeSigningKey {
        #[structopt(long = "key-id", short, help = "Signing Key ID")]
        key_id: String,
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },
}

#[derive(Debug, StructOpt)]
enum CacheCommand {
    #[structopt(about = "Creates a Momento Cache")]
    Create {
        #[structopt(long = "name", short = 'n')]
        cache_name: String,
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Stores a given item in cache")]
    Set {
        #[structopt(long = "name", short = 'n')]
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
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Gets item from the cache")]
    Get {
        #[structopt(long = "name", short = 'n')]
        cache_name: Option<String>,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Deletes the cache")]
    Delete {
        #[structopt(long = "name", short = 'n')]
        cache_name: String,
        #[structopt(long, short, default_value = "default")]
        profile: String,
    },

    #[structopt(about = "Lists all momento caches")]
    List {
        #[structopt(long, short, default_value = "default")]
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
                debug!("created cache {cache_name}")
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
                debug!("deleted cache {}", cache_name)
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
            AccountCommand::CreateSigningKey {
                ttl_minutes,
                profile,
            } => {
                let (creds, _config) = get_creds_and_config(&profile).await?;
                commands::signingkey::signingkey_cli::create_signing_key(ttl_minutes, creds.token)
                    .await?;
            }
            AccountCommand::RevokeSigningKey { key_id, profile } => {
                let (creds, _config) = get_creds_and_config(&profile).await?;
                commands::signingkey::signingkey_cli::revoke_signing_key(
                    key_id.clone(),
                    creds.token,
                )
                .await?;
                debug!("revoked signing key {}", key_id)
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
