use env_logger::Env;
use home::home_dir;
use std::{
    env::{self},
    path::Path,
};
use structopt::StructOpt;
use tokio::{
    fs::{self, File},
    io::{self, stdin, stdout, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
};

mod commands;
mod credentials;
mod utils;

#[derive(StructOpt)]
#[structopt(about = "CLI for Momento APIs")]
struct Momento {
    #[structopt(name = "verbose", global = true, long)]
    verbose: bool,

    #[structopt(subcommand)]
    command: Subcommand,
}

#[derive(StructOpt)]
enum Subcommand {
    #[structopt(about = "Cache Operations")]
    Cache {
        #[structopt(subcommand)]
        operation: CacheCommand,
    },
    #[structopt(about = "Configure Momento Credentials")]
    Configure {},
}

#[derive(StructOpt)]
enum CacheCommand {
    #[structopt(about = "Creates a Momento Cache")]
    Create {
        #[structopt(name = "name", long, short)]
        cache_name: String,
    },

    #[structopt(about = "Stores a given item in cache")]
    Set {
        #[structopt(name = "name", long, short)]
        cache_name: String,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,
        #[structopt(long, short)]
        value: String,
        #[structopt(long = "ttl_seconds", short = "ttl")]
        ttl_seconds: u32,
    },

    #[structopt(about = "Gets item from the cache")]
    Get {
        #[structopt(name = "name", long, short)]
        cache_name: String,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,
    },

    #[structopt(about = "Deletes the cache")]
    Delete {
        #[structopt(name = "name", long, short)]
        cache_name: String,
    },
}

#[tokio::main]
async fn main() {
    let args = Momento::from_args();

    let log_level = if args.verbose { "debug" } else { "info" };

    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or(log_level)
            .default_write_style_or("always"),
    )
    .init();

    // TODO: Make this more robust, read from profile etc.
    let auth_token = env::var("MOMENTO_AUTH_TOKEN").expect("MOMENTO_AUTH_TOKEN must be set");

    match args.command {
        Subcommand::Cache { operation } => match operation {
            CacheCommand::Create { cache_name } => {
                commands::cache::cache::create_cache(cache_name, auth_token).await
            }
            CacheCommand::Set {
                cache_name,
                key,
                value,
                ttl_seconds,
            } => commands::cache::cache::set(cache_name, auth_token, key, value, ttl_seconds).await,
            CacheCommand::Get { cache_name, key } => {
                commands::cache::cache::get(cache_name, auth_token, key).await
            }
            CacheCommand::Delete { cache_name } => {
                commands::cache::cache::delete_cache(cache_name, auth_token).await
            }
        },
        Subcommand::Configure {} => commands::configure::configure::configure_momento().await,
    }
}
