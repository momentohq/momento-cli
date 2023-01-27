use std::{panic, process::exit};

use clap::Parser;
#[cfg(feature = "login")]
use commands::login::LoginMode;
use commands::topic::print_subscription;
use env_logger::Env;
use error::CliError;
use log::{debug, error, LevelFilter};
use utils::user::get_creds_and_config;

use crate::utils::console::console_info;
#[cfg(feature = "login")]
use crate::utils::user::clobber_session_token;

mod commands;
mod config;
mod error;
mod utils;

#[derive(Debug, Parser)]
#[clap(version, about = "CLI for Momento APIs", name = "momento")]
struct Momento {
    #[arg(name = "verbose", global = true, long, help = "Log more information")]
    verbose: bool,

    #[arg(
        long,
        short,
        default_value = "default",
        global = true,
        help = "User profile"
    )]
    profile: String,

    #[command(subcommand)]
    command: Subcommand,
}

#[derive(Debug, Parser)]
enum Subcommand {
    #[command(about = "Interact with caches")]
    Cache {
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,

        #[command(subcommand)]
        operation: CacheCommand,
    },
    /// Interact with topics
    /// !!                            !!
    /// !!       Preview feature      !!
    /// !!  Your feedback is welcome  !!
    /// !!                            !!
    /// These commands requires a cache, which serves as a namespace
    /// for your topics. If you haven't already, call `cache create`
    /// to make one!
    ///
    /// To create a topic, subscribe to it.
    /// To delete a topic, stop subscribing to it.
    #[command(verbatim_doc_comment, hide = true)]
    Topic {
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,

        #[command(subcommand)]
        operation: TopicCommand,
    },
    #[command(about = "Configure credentials")]
    Configure {
        #[arg(long, short)]
        quick: bool,
    },
    #[command(about = "Manage accounts")]
    Account {
        #[command(subcommand)]
        operation: AccountCommand,
    },
    #[command(about = "Manage signing keys")]
    SigningKey {
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,

        #[command(subcommand)]
        operation: SigningKeyCommand,
    },
    #[cfg(feature = "login")]
    #[command(
        about = "*Construction Zone* We're working on this! *Construction Zone* Log in to manage your Momento account"
    )]
    Login {
        #[arg(value_enum, default_value = "browser")]
        via: LoginMode,
    },
}

#[derive(Debug, Parser)]
enum SigningKeyCommand {
    #[command(about = "Create a signing key")]
    Create {
        #[arg(
            long = "ttl",
            short = 't',
            default_value = "86400",
            help = "Duration, in minutes, that the signing key will be valid"
        )]
        ttl_minutes: u32,
    },

    #[command(about = "Revoke the signing key")]
    Revoke {
        #[arg(long = "key-id", short, help = "Signing Key ID")]
        key_id: String,
    },

    #[command(about = "List all signing keys")]
    List {},
}

#[derive(Debug, Parser)]
enum AccountCommand {
    #[command(about = "Sign up for Momento")]
    Signup {
        #[command(subcommand)]
        signup_operation: CloudSignupCommand,
    },
}

#[derive(Debug, Parser)]
enum CloudSignupCommand {
    #[command(about = "Signup for Momento on GCP")]
    Gcp {
        #[arg(long, short)]
        email: String,
        #[arg(long, short, value_name = "us-east1 or asia-northeast1")]
        region: String,
    },
    #[command(about = "Signup for Momento on AWS")]
    Aws {
        #[arg(long, short)]
        email: String,
        #[arg(long, short, value_name = "us-west-2, us-east-1, or ap-northeast-1")]
        region: String,
    },
}

#[derive(Debug, Parser)]
enum CacheCommand {
    #[command(
        about = "Create a cache",
        group(
            clap::ArgGroup::new("cache-name")
                .required(true)
                .args(["cache_name", "cache_name_flag", "cache_name_flag_for_backward_compatibility"]),
        ),
    )]
    Create {
        #[arg(
            help = "Name of the cache you want to create. Must be at least 3 characters and unique within your account.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,

        #[arg(long = "cache", value_name = "CACHE")]
        cache_name_flag: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,
    },

    #[command(
        about = "Delete a cache",
        group(
            clap::ArgGroup::new("cache-name")
                .required(true)
                .args(["cache_name", "cache_name_flag", "cache_name_flag_for_backward_compatibility"]),
        ),
    )]
    Delete {
        #[arg(help = "Name of the cache you want to delete.", value_name = "CACHE")]
        cache_name: Option<String>,

        #[arg(long = "cache", value_name = "CACHE")]
        cache_name_flag: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,
    },

    #[command(about = "List all caches")]
    List {},

    #[command(
        about = "Store an item in a cache",
        group(
            clap::ArgGroup::new("cache-key")
                .required(true)
                .args(["key", "key_flag"]),
        ),
        group(
            clap::ArgGroup::new("cache-value")
                .required(true)
                .args(["value", "value_flag"]),
        ),
        group(
            clap::ArgGroup::new("cache-name")
                .args(["cache_name", "cache_name_flag_for_backward_compatibility"]),
        ),
    )]
    Set {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,

        // TODO: Add support for non-string key-value
        #[arg(help = "Cache key under which to store the value")]
        key: Option<String>,
        #[arg(long = "key", value_name = "KEY")]
        key_flag: Option<String>,

        #[arg(help = "Cache value to store under the key. This will be stored as UTF-8 bytes.")]
        value: Option<String>,
        #[arg(long = "value", value_name = "VALUE")]
        value_flag: Option<String>,

        #[arg(
            long = "ttl",
            help = "Max time, in seconds, that the item will be stored in cache"
        )]
        ttl_seconds: Option<u64>,
    },

    #[command(
        about = "Get an item from the cache",
        group(
            clap::ArgGroup::new("cache-key")
                .required(true)
                .args(["key", "key_flag"]),
        ),
        group(
            clap::ArgGroup::new("cache-name")
                .args(["cache_name", "cache_name_flag_for_backward_compatibility"]),
        ),
    )]
    Get {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,

        // TODO: Add support for non-string key-value
        #[arg(help = "Cache key under which to store the value")]
        key: Option<String>,
        #[arg(long = "key", value_name = "KEY")]
        key_flag: Option<String>,
    },
}

#[derive(Debug, Parser)]
enum TopicCommand {
    /// Publish a value to all subscribers of a topic.
    #[command()]
    Publish {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use as your topic namespace. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,

        #[arg(help = "Name of the topic to which you would like to publish")]
        topic: String,
        #[arg(help = "String message value to publish")]
        value: String,
    },

    /// Subscribe to messages coming in on a topic.
    #[command()]
    Subscribe {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use as your topic namespace. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,

        #[arg(help = "Name of the topic to which you would like to subscribe")]
        topic: String,
    },
}

async fn run_momento_command(args: Momento) -> Result<(), CliError> {
    match args.command {
        Subcommand::Cache {
            endpoint,
            operation,
        } => match operation {
            CacheCommand::Create {
                cache_name_flag,
                cache_name,
                cache_name_flag_for_backward_compatibility,
            } => {
                let cache_name = cache_name
                    .or(cache_name_flag)
                    .or(cache_name_flag_for_backward_compatibility)
                    .expect("The argument group guarantees 1 or the other");
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::create_cache(cache_name.clone(), creds.token, endpoint)
                    .await?;
                debug!("created cache {cache_name}")
            }
            CacheCommand::Delete {
                cache_name,
                cache_name_flag,
                cache_name_flag_for_backward_compatibility,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                let cache_name = cache_name
                    .or(cache_name_flag)
                    .or(cache_name_flag_for_backward_compatibility)
                    .expect("The argument group guarantees 1 or the other");
                commands::cache::cache_cli::delete_cache(cache_name.clone(), creds.token, endpoint)
                    .await?;
                debug!("deleted cache {}", cache_name)
            }
            CacheCommand::List {} => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::list_caches(creds.token, endpoint).await?
            }
            CacheCommand::Set {
                cache_name,
                cache_name_flag_for_backward_compatibility,
                key,
                key_flag,
                value,
                value_flag,
                ttl_seconds,
            } => {
                let (creds, config) = get_creds_and_config(&args.profile).await?;
                let cache_name = cache_name
                    .or(cache_name_flag_for_backward_compatibility)
                    .unwrap_or(config.cache);
                let key = key
                    .or(key_flag)
                    .expect("The argument group guarantees 1 or the other");
                let value = value
                    .or(value_flag)
                    .expect("The argument group guarantees 1 or the other");
                commands::cache::cache_cli::set(
                    cache_name,
                    creds.token,
                    key,
                    value,
                    ttl_seconds.unwrap_or(config.ttl),
                    endpoint,
                )
                .await?
            }
            CacheCommand::Get {
                cache_name,
                cache_name_flag_for_backward_compatibility,
                key,
                key_flag,
            } => {
                let (creds, config) = get_creds_and_config(&args.profile).await?;
                let key = key
                    .or(key_flag)
                    .expect("The argument group guarantees 1 or the other");
                commands::cache::cache_cli::get(
                    cache_name
                        .or(cache_name_flag_for_backward_compatibility)
                        .unwrap_or(config.cache),
                    creds.token,
                    key,
                    endpoint,
                )
                .await?;
            }
        },
        Subcommand::Topic {
            endpoint,
            operation,
        } => {
            let (creds, config) = get_creds_and_config(&args.profile).await?;
            let mut client =
                momento::preview::topics::TopicClient::connect(creds.token, endpoint, Some("cli"))
                    .map_err(|e| CliError {
                        msg: format!("could not connect: {e:#?}"),
                    })?;
            match operation {
                TopicCommand::Publish {
                    cache_name,
                    topic,
                    value,
                } => {
                    let cache_name = cache_name.unwrap_or(config.cache);
                    client
                        .publish_mut(cache_name, topic, value)
                        .await
                        .map_err(|e| CliError {
                            msg: format!("could not publish: {e:?}"),
                        })?;
                }
                TopicCommand::Subscribe { cache_name, topic } => {
                    let cache_name = cache_name.unwrap_or(config.cache);
                    let subscription = client
                        .subscribe_mut(cache_name, topic, None)
                        .await
                        .map_err(|e| CliError {
                            msg: format!("could not subscribe: {e:#?}"),
                        })?;
                    print_subscription(subscription)
                        .await
                        .map_err(|e| CliError {
                            msg: format!("subscription error: {e:?}"),
                        })?;
                }
            }
        }
        Subcommand::Configure { quick } => {
            commands::configure::configure_cli::configure_momento(quick, &args.profile).await?
        }
        Subcommand::Account { operation } => match operation {
            AccountCommand::Signup { signup_operation } => match signup_operation {
                CloudSignupCommand::Gcp { email, region } => {
                    commands::account::signup_user(email, "gcp".to_string(), region).await?
                }
                CloudSignupCommand::Aws { email, region } => {
                    commands::account::signup_user(email, "aws".to_string(), region).await?
                }
            },
        },
        Subcommand::SigningKey {
            endpoint,
            operation,
        } => match operation {
            SigningKeyCommand::Create { ttl_minutes } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::signingkey::signingkey_cli::create_signing_key(
                    ttl_minutes,
                    creds.token,
                    endpoint,
                )
                .await?;
            }
            SigningKeyCommand::Revoke { key_id } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::signingkey::signingkey_cli::revoke_signing_key(
                    key_id.clone(),
                    creds.token,
                    endpoint,
                )
                .await?;
                debug!("revoked signing key {}", key_id)
            }
            SigningKeyCommand::List {} => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::signingkey::signingkey_cli::list_signing_keys(creds.token, endpoint)
                    .await?
            }
        },
        #[cfg(feature = "login")]
        Subcommand::Login { via } => match commands::login::login(via).await {
            Ok(credentials) => {
                let session_token = credentials.token();
                let session_duration = credentials.valid_for();
                debug!("{session_token}");
                clobber_session_token(
                    Some(session_token.to_string()),
                    session_duration.as_secs() as u32,
                )
                .await?;
                console_info!("Login valid for {}m", session_duration.as_secs() / 60);
            }
            Err(auth_error) => {
                return Err(CliError {
                    msg: format!("auth error: {auth_error:?}"),
                })
            }
        },
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Momento::parse();

    let log_level = if args.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Error
    }
    .as_str();

    panic::set_hook(Box::new(move |info| {
        error!("{}", info);
    }));

    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or(log_level)
            .default_write_style_or("always"),
    )
    .init();

    if let Err(e) = run_momento_command(args).await {
        console_info!("{}", e);
        exit(1)
    }
}
