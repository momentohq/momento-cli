use std::{panic, process::exit};

use clap::Parser;
use commands::topic::print_subscription;
use env_logger::Env;
use error::CliError;
use log::{debug, error, LevelFilter};
use momento::{topics, FunctionClient, MomentoError, TopicClient};
use momento_cli_opts::PreviewCommand;
use utils::{console::output_info, user::get_creds_and_config};

use crate::{commands::functions::utils::determine_wasm_source, utils::console::console_info};

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
                commands::cache::cache_cli::create_cache(cache_name.clone(), creds, endpoint)
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
                commands::cache::cache_cli::delete_cache(cache_name.clone(), creds, endpoint)
                    .await?;
                debug!("deleted cache {}", cache_name)
            }
            momento_cli_opts::CacheCommand::List {} => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::list_caches(creds, endpoint).await?
            }
            momento_cli_opts::CacheCommand::Flush {
                cache_name,
                cache_name_flag,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                let cache_name = cache_name
                    .or(cache_name_flag)
                    .expect("The argument group guarantees 1 or the other");
                commands::cache::cache_cli::flush_cache(cache_name, creds, endpoint).await?
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
                    creds,
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
                    creds,
                    key,
                    endpoint,
                )
                .await?;
            }
            momento_cli_opts::CacheCommand::DeleteItem {
                cache_name,
                cache_name_flag_for_backward_compatibility,
                key,
                key_flag,
            } => {
                let (creds, config) = get_creds_and_config(&args.profile).await?;
                let key = key
                    .or(key_flag)
                    .expect("The argument group guarantees 1 or the other");
                commands::cache::cache_cli::delete_key(
                    cache_name
                        .or(cache_name_flag_for_backward_compatibility)
                        .unwrap_or(config.cache),
                    creds,
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
            let mut credential_provider = creds.authenticate()?;
            if let Some(endpoint_override) = endpoint {
                credential_provider = credential_provider.base_endpoint(&endpoint_override);
            }

            let client = TopicClient::builder()
                .configuration(topics::configurations::Laptop::latest())
                .credential_provider(credential_provider)
                .build()
                .map_err(Into::<CliError>::into)?;
            match operation {
                momento_cli_opts::TopicCommand::Publish {
                    cache_name,
                    topic,
                    value,
                } => {
                    let cache_name = cache_name.unwrap_or(config.cache);
                    client
                        .publish(cache_name, topic, value)
                        .await
                        .map_err(Into::<CliError>::into)?;
                }
                momento_cli_opts::TopicCommand::Subscribe { cache_name, topic } => {
                    let cache_name = cache_name.unwrap_or(config.cache);
                    let subscription =
                        client
                            .subscribe(cache_name, topic)
                            .await
                            .map_err(|e| CliError {
                                msg: format!(
                                    "the subscription ended without receiving any values: {e:?}"
                                ),
                            })?;
                    match print_subscription(subscription).await {
                        Ok(_) => console_info!("The subscription ended"),
                        Err(e) => {
                            output_info(&format!("The subscription ended: {}", e.message));
                            console_info!("detail: {}", e.message);
                            return Err(e.into());
                        }
                    }
                }
            }
        }
        momento_cli_opts::Subcommand::Configure {
            quick,
            api_key_and_endpoint,
            disposable_token,
        } => {
            commands::configure::configure_cli::configure_momento(
                quick,
                &args.profile,
                api_key_and_endpoint,
                disposable_token,
            )
            .await?
        }
        momento_cli_opts::Subcommand::Account { operation } => match operation {
            // This command has been removed. It now just prints out an error message.
            momento_cli_opts::AccountCommand::Signup {
                signup_operation: _,
            } => commands::account::signup_decommissioned().await?,
        },
        momento_cli_opts::Subcommand::Preview { operation } => match operation {
            PreviewCommand::CloudLinter {
                region,
                enable_ddb_ttl_check,
                resource,
                metric_collection_rate,
                enable_gsi,
                enable_s3,
                enable_api_gateway,
                metric_start_date,
                metric_end_date,
            } => {
                commands::cloud_linter::linter_cli::run_cloud_linter(
                    region,
                    enable_ddb_ttl_check,
                    enable_gsi,
                    enable_s3,
                    enable_api_gateway,
                    resource,
                    metric_collection_rate,
                    metric_start_date,
                    metric_end_date,
                )
                .await?;
            }
            PreviewCommand::Function { operation } => {
                let (creds, _) = get_creds_and_config(&args.profile).await?;
                let credential_provider = creds.authenticate()?;
                let endpoint = credential_provider.cache_http_endpoint().to_string();
                let auth_token = credential_provider.auth_token().to_string();
                let client = FunctionClient::builder()
                    .credential_provider(credential_provider)
                    .build()
                    .map_err(Into::<CliError>::into)?;

                match operation {
                    momento_cli_opts::FunctionCommand::PutFunction {
                        cache_name,
                        name,
                        wasm_file,
                        id_uploaded_wasm,
                        version_uploaded_wasm,
                        description,
                        environment_variables,
                    } => {
                        let wasm_source = determine_wasm_source(
                            wasm_file,
                            id_uploaded_wasm,
                            version_uploaded_wasm,
                        )?;
                        commands::functions::function_cli::put_function(
                            client,
                            cache_name,
                            name,
                            wasm_source,
                            description,
                            environment_variables,
                        )
                        .await?
                    }
                    momento_cli_opts::FunctionCommand::PutWasm {
                        name,
                        wasm_file,
                        description,
                    } => {
                        commands::functions::function_cli::put_wasm(
                            client,
                            name,
                            wasm_file,
                            description,
                        )
                        .await?
                    }
                    momento_cli_opts::FunctionCommand::InvokeFunction {
                        cache_name,
                        name,
                        data,
                    } => {
                        commands::functions::function_cli::invoke_function(
                            endpoint, auth_token, cache_name, name, data,
                        )
                        .await?
                    }
                    momento_cli_opts::FunctionCommand::ListFunctions { cache_name } => {
                        commands::functions::function_cli::list_functions(client, cache_name)
                            .await?
                    }
                    momento_cli_opts::FunctionCommand::ListFunctionVersions { function_id } => {
                        commands::functions::function_cli::list_function_versions(
                            client,
                            function_id,
                        )
                        .await?
                    }
                    momento_cli_opts::FunctionCommand::ListWasms {} => {
                        commands::functions::function_cli::list_wasms(client).await?
                    }
                }
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
