use env_logger::Env;
use structopt::StructOpt;

mod commands;

#[derive(StructOpt)]
#[structopt(about = "CLI for Momento APIs")]
struct Momento {
    #[structopt(name = "verbose", global = true, long, short)]
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
        // TODO: Read from profile
        #[structopt()]
        auth_token: String,
    },
}

#[derive(StructOpt)]
enum CacheCommand {
    #[structopt(about = "Creates a Momento Cache")]
    Create {
        #[structopt()]
        cache_name: String,
    },

    #[structopt(about = "Stores a given item in cache")]
    Set {
        #[structopt()]
        cache_name: String,
        // TODO: Add support for non-string key-value
        #[structopt()]
        key: String,
        #[structopt()]
        value: String,
        #[structopt()]
        ttl_seconds: u32,
    },

    #[structopt(about = "Gets item from the cache")]
    Get {
        #[structopt()]
        cache_name: String,
        // TODO: Add support for non-string key-value
        #[structopt()]
        key: String,
    },

    #[structopt(about = "Deletes the cache")]
    Delete {
        #[structopt(long)]
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

    match args.command {
        Subcommand::Cache { operation, auth_token } => match operation {
            CacheCommand::Create {
                cache_name,
            } => {
                commands::cache::cache::create_cache(cache_name, auth_token).await
            }
            CacheCommand::Set { cache_name, key, value, ttl_seconds } => {
                commands::cache::cache::set(cache_name, auth_token, key, value, ttl_seconds).await
            }
            CacheCommand::Get { cache_name, key } => {
                commands::cache::cache::get(cache_name, auth_token, key).await
            }
            CacheCommand::Delete { cache_name } => {
                commands::cache::cache::delete_cache(cache_name, auth_token).await
            }
        },
    }
}
