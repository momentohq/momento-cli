use std::{panic, process::exit};

use clap::Parser;
use commands::topic::print_subscription;
use env_logger::Env;
use error::CliError;
use log::{debug, error, LevelFilter};
use momento::{CredentialProviderBuilder, MomentoError};
use utils::{console::output_info, user::get_creds_and_config};

use crate::utils::console::console_info;
use crate::utils::user::clobber_session_token;

mod commands;
mod config;
mod error;
mod utils;

async fn run_momento_command(args: momento_cli_opts::Momento) -> Result<(), CliError> {
    match args.command {
        momento_cli_opts::Subcommand::Cache {
            endpoint,
            operation,
        } => match operation {
            momento_cli_opts::CacheCommand::Create {
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
            momento_cli_opts::CacheCommand::Delete {
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
            momento_cli_opts::CacheCommand::List {} => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::list_caches(creds.token, endpoint).await?
            }
            momento_cli_opts::CacheCommand::Flush {
                cache_name,
                cache_name_flag,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                let cache_name = cache_name
                    .or(cache_name_flag)
                    .expect("The argument group guarantees 1 or the other");
                commands::cache::cache_cli::flush_cache(cache_name, creds.token, endpoint).await?
            }
            momento_cli_opts::CacheCommand::Set {
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
            momento_cli_opts::CacheCommand::Get {
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
        momento_cli_opts::Subcommand::Topic {
            endpoint,
            operation,
        } => {
            let (creds, config) = get_creds_and_config(&args.profile).await?;
            let mut credential_provider_builder =
                CredentialProviderBuilder::from_string(creds.token);
            if let Some(endpoint_override) = endpoint {
                credential_provider_builder =
                    credential_provider_builder.with_momento_endpoint(endpoint_override)
            }
            let credential_provider = credential_provider_builder.build()?;

            let mut client =
                momento::preview::topics::TopicClient::connect(credential_provider, Some("cli"))
                    .map_err(Into::<CliError>::into)?;
            match operation {
                momento_cli_opts::TopicCommand::Publish {
                    cache_name,
                    topic,
                    value,
                } => {
                    let cache_name = cache_name.unwrap_or(config.cache);
                    client
                        .publish_mut(cache_name, topic, value)
                        .await
                        .map_err(Into::<CliError>::into)?;
                }
                momento_cli_opts::TopicCommand::Subscribe { cache_name, topic } => {
                    let cache_name = cache_name.unwrap_or(config.cache);
                    let subscription =
                        client
                            .subscribe(cache_name, topic, None)
                            .await
                            .map_err(|e| CliError {
                                msg: format!(
                                    "the subscription ended without receiving any values: {e:?}"
                                ),
                            })?;
                    match print_subscription(subscription).await {
                        Ok(_) => console_info!("The subscription ended"),
                        Err(e) => match e {
                            momento::MomentoError::Interrupted {
                                description,
                                source,
                            } => {
                                output_info(&format!("The subscription ended: {description}"));
                                console_info!("detail: {source:?}");
                            }
                            _ => return Err(e.into()),
                        },
                    }
                }
            }
        }
        momento_cli_opts::Subcommand::Configure { quick } => {
            commands::configure::configure_cli::configure_momento(quick, &args.profile).await?
        }
        momento_cli_opts::Subcommand::Account { operation } => match operation {
            momento_cli_opts::AccountCommand::Signup { signup_operation } => match signup_operation
            {
                momento_cli_opts::CloudSignupCommand::Gcp { email, region } => {
                    commands::account::signup_user(email, "gcp".to_string(), region).await?
                }
                momento_cli_opts::CloudSignupCommand::Aws { email, region } => {
                    commands::account::signup_user(email, "aws".to_string(), region).await?
                }
            },
        },
        momento_cli_opts::Subcommand::Preview { operation } => match operation {
            momento_cli_opts::PreviewCommand::SigningKey {
                endpoint,
                operation,
            } => match operation {
                momento_cli_opts::SigningKeyCommand::Create { ttl_minutes } => {
                    let (creds, _config) = get_creds_and_config(&args.profile).await?;
                    commands::signingkey::signingkey_cli::create_signing_key(
                        ttl_minutes,
                        creds.token,
                        endpoint,
                    )
                    .await?;
                }
                momento_cli_opts::SigningKeyCommand::Revoke { key_id } => {
                    let (creds, _config) = get_creds_and_config(&args.profile).await?;
                    commands::signingkey::signingkey_cli::revoke_signing_key(
                        key_id.clone(),
                        creds.token,
                        endpoint,
                    )
                    .await?;
                    debug!("revoked signing key {}", key_id)
                }
                momento_cli_opts::SigningKeyCommand::List {} => {
                    let (creds, _config) = get_creds_and_config(&args.profile).await?;
                    commands::signingkey::signingkey_cli::list_signing_keys(creds.token, endpoint)
                        .await?
                }
            },
            momento_cli_opts::PreviewCommand::Login { via } => {
                match commands::login::login(via).await {
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
                }
            }
            momento_cli_opts::PreviewCommand::GenerateToken {
                valid_for,
                never_expire,
                endpoint,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::tokens::generate_api_token(
                    creds.token,
                    endpoint,
                    never_expire,
                    valid_for,
                )
                .await?;
            }
        },
    }
    Ok(())
}

/// todo: fix CliError to either not exist anymore or actually support sources
/// todo: pick output strings more intentionally
impl From<MomentoError> for CliError {
    fn from(val: MomentoError) -> Self {
        CliError {
            msg: format!("{val:?}"),
        }
    }
}

#[tokio::main]
async fn main() {
    let args = momento_cli_opts::Momento::parse();

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
