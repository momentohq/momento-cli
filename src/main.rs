use std::panic;

use env_logger::Env;
use log::error;
use structopt::StructOpt;
use utils::get_creds_for_profile;

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
    #[structopt(about = "Account Operations")]
    Account {
        #[structopt(subcommand)]
        operation: AccountCommand,
    },
}

#[derive(StructOpt)]
enum AccountCommand {
    #[structopt(about = "Signs a user up for Momento and emails your token to the provided email")]
    Signup {
        #[structopt(name = "email", long, short)]
        email: String,
    },
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
        #[structopt(
            long = "ttl",
            short = "ttl",
            default_value = "300",
            help = "Max time, in seconds, that the item will be stored in cache"
        )]
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
    panic::set_hook(Box::new(move |info| {
        error!("{}", info);
    }));

    let args = Momento::from_args();

    let log_level = if args.verbose { "debug" } else { "info" };

    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or(log_level)
            .default_write_style_or("always"),
    )
    .init();

    match args.command {
        Subcommand::Cache { operation } => match operation {
            CacheCommand::Create { cache_name } => {
                let creds = get_creds_for_profile(None).await;
                commands::cache::cache::create_cache(cache_name, creds.token).await
            }
            CacheCommand::Set {
                cache_name,
                key,
                value,
                ttl_seconds,
            } => {
                let creds = get_creds_for_profile(None).await;
                commands::cache::cache::set(cache_name, creds.token, key, value, ttl_seconds).await
            }
            CacheCommand::Get { cache_name, key } => {
                let creds = get_creds_for_profile(None).await;
                commands::cache::cache::get(cache_name, creds.token, key).await;
            }
            CacheCommand::Delete { cache_name } => {
                let creds = get_creds_for_profile(None).await;
                commands::cache::cache::delete_cache(cache_name, creds.token).await
            }
        },
        Subcommand::Configure {} => commands::configure::configure::configure_momento().await,
        Subcommand::Account { operation } => match operation {
            AccountCommand::Signup { email } => {
                commands::signup::signup::signup_user(email).await;
            }
        },
    }
}
