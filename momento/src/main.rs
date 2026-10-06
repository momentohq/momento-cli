use std::{panic, process::exit};

use clap::Parser;
use commands::topic::print_subscription;
use env_logger::Env;
use error::CliError;
use log::{debug, error, warn, LevelFilter};
use momento::MomentoError;
use momento_cli_opts::PreviewCommand;
use std::time::{SystemTime, UNIX_EPOCH};
use utils::{
    client::{get_cache_client, get_function_client, get_topic_client},
    console::output_info,
    user::{determine_mga_endpoint, get_creds_and_config},
};

use crate::{
    commands::api_key::utils::determine_expiry,
    commands::capacity_pool::utils::{determine_provisioning, determine_provisioning_update},
    commands::custom_role::utils::{determine_role, determine_role_selector},
    commands::functions::utils::{
        determine_current_function_version, determine_function_metrics_config_change,
        determine_wasm_source, InvocationOptions,
    },
    commands::utils::determine_metrics_config,
    utils::console::console_info,
};

mod commands;
mod config;
mod error;
mod utils;

async fn run_momento_command(args: momento_cli_opts::Momento) -> Result<(), CliError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            CliError::new("The system clock is before 1970.").with_details(error.to_string())
        })?
        .as_secs();
    match args.command {
        momento_cli_opts::Subcommand::ApiKey { api_key, operation } => match operation {
            momento_cli_opts::ApiKeyCommand::AuthenticatedSubcommand(operation) => {
                let (creds, _) = get_creds_and_config(&args.profile).await?;
                let credential_provider = creds.override_and_authenticate(api_key, None)?;

                let mga_endpoint =
                    determine_mga_endpoint(credential_provider.cache_http_endpoint().to_string());
                let auth_token = credential_provider.auth_token().to_string();

                match operation {
                    momento_cli_opts::AuthenticatedApiKeyCommand::Create {
                        description,
                        role_name,
                        role_id,
                        expires_at_epoch_seconds,
                        expires_in,
                        expires_on,
                        exclude_refresh_token,
                    } => {
                        let role_selector = determine_role_selector(role_id, role_name)?;
                        let role = determine_role(
                            mga_endpoint.clone(),
                            auth_token.clone(),
                            &role_selector,
                            true,
                        )
                        .await?;
                        let expiry = determine_expiry(
                            expires_at_epoch_seconds,
                            expires_in,
                            expires_on,
                            now,
                        )?;

                        commands::api_key::key_cli::create_key(
                            mga_endpoint,
                            auth_token,
                            description,
                            role.id,
                            expiry,
                            exclude_refresh_token,
                        )
                        .await?
                    }
                    momento_cli_opts::AuthenticatedApiKeyCommand::Refresh {
                        refresh_token,
                        expires_at_epoch_seconds,
                        expires_in,
                        expires_on,
                    } => {
                        let expiry = determine_expiry(
                            expires_at_epoch_seconds,
                            expires_in,
                            expires_on,
                            now,
                        )?;

                        commands::api_key::key_cli::refresh_key(mga_endpoint, refresh_token, expiry)
                            .await?
                    }
                    momento_cli_opts::AuthenticatedApiKeyCommand::Revoke { id } => {
                        commands::api_key::key_cli::revoke_key(mga_endpoint, auth_token, id).await?
                    }
                    momento_cli_opts::AuthenticatedApiKeyCommand::List { limit } => {
                        commands::api_key::key_cli::list_keys(mga_endpoint, auth_token, limit)
                            .await?
                    }
                }
            }
            momento_cli_opts::ApiKeyCommand::Decode { api_key } => {
                commands::api_key::key_cli::decode_key(api_key)?
            }
        },
        momento_cli_opts::Subcommand::Role { api_key, operation } => {
            let (creds, _) = get_creds_and_config(&args.profile).await?;
            let credential_provider = creds.override_and_authenticate(api_key, None)?;

            let mga_endpoint =
                determine_mga_endpoint(credential_provider.cache_http_endpoint().to_string());
            let auth_token = credential_provider.auth_token().to_string();

            match operation {
                momento_cli_opts::CustomRoleCommand::Create {
                    name,
                    description,
                    permission_set,
                } => {
                    commands::custom_role::role_cli::create_role(
                        mga_endpoint,
                        auth_token,
                        name,
                        description,
                        permission_set,
                    )
                    .await?
                }
                momento_cli_opts::CustomRoleCommand::Update {
                    id,
                    name,
                    new_name,
                    description,
                    permission_set,
                } => {
                    let selector = determine_role_selector(id, name)?;
                    commands::custom_role::role_cli::update_role(
                        mga_endpoint,
                        auth_token,
                        selector,
                        new_name,
                        description,
                        permission_set,
                    )
                    .await?
                }
                momento_cli_opts::CustomRoleCommand::Delete { id, name } => {
                    let selector = determine_role_selector(id, name)?;
                    commands::custom_role::role_cli::delete_role(mga_endpoint, auth_token, selector)
                        .await?
                }
                momento_cli_opts::CustomRoleCommand::List { limit, all } => {
                    commands::custom_role::role_cli::list_roles(
                        mga_endpoint,
                        auth_token,
                        limit,
                        all,
                    )
                    .await?
                }
            }
        }
        momento_cli_opts::Subcommand::Cache {
            api_key,
            endpoint,
            operation,
        } => {
            let (creds, config) = get_creds_and_config(&args.profile).await?;
            let credential_provider = creds.override_and_authenticate(api_key, endpoint)?;
            let client = get_cache_client(credential_provider).await?;

            match operation {
                momento_cli_opts::CacheCommand::Create {
                    cache_name_flag,
                    cache_name,
                    cache_name_flag_for_backward_compatibility,
                } => {
                    let cache_name = cache_name
                        .or(cache_name_flag)
                        .or(cache_name_flag_for_backward_compatibility)
                        .expect("The argument group guarantees 1 or the other");
                    commands::cache::cache_cli::create_cache(client, cache_name.clone()).await?;
                    debug!("created cache {cache_name}")
                }
                momento_cli_opts::CacheCommand::Delete {
                    cache_name,
                    cache_name_flag,
                    cache_name_flag_for_backward_compatibility,
                } => {
                    let cache_name = cache_name
                        .or(cache_name_flag)
                        .or(cache_name_flag_for_backward_compatibility)
                        .expect("The argument group guarantees 1 or the other");
                    commands::cache::cache_cli::delete_cache(client, cache_name.clone()).await?;
                    debug!("deleted cache {}", cache_name)
                }
                momento_cli_opts::CacheCommand::List {} => {
                    commands::cache::cache_cli::list_caches(client).await?
                }
                momento_cli_opts::CacheCommand::Flush {
                    cache_name,
                    cache_name_flag,
                } => {
                    let cache_name = cache_name
                        .or(cache_name_flag)
                        .expect("The argument group guarantees 1 or the other");
                    commands::cache::cache_cli::flush_cache(client, cache_name).await?
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
                        client,
                        cache_name,
                        key,
                        value,
                        ttl_seconds.unwrap_or(config.ttl),
                    )
                    .await?
                }
                momento_cli_opts::CacheCommand::Get {
                    cache_name,
                    cache_name_flag_for_backward_compatibility,
                    key,
                    key_flag,
                } => {
                    let key = key
                        .or(key_flag)
                        .expect("The argument group guarantees 1 or the other");
                    commands::cache::cache_cli::get(
                        client,
                        cache_name
                            .or(cache_name_flag_for_backward_compatibility)
                            .unwrap_or(config.cache),
                        key,
                    )
                    .await?;
                }
                momento_cli_opts::CacheCommand::DeleteItem {
                    cache_name,
                    cache_name_flag_for_backward_compatibility,
                    key,
                    key_flag,
                } => {
                    let key = key
                        .or(key_flag)
                        .expect("The argument group guarantees 1 or the other");
                    commands::cache::cache_cli::delete_key(
                        client,
                        cache_name
                            .or(cache_name_flag_for_backward_compatibility)
                            .unwrap_or(config.cache),
                        key,
                    )
                    .await?;
                }
            }
        }
        momento_cli_opts::Subcommand::Topic {
            api_key,
            endpoint,
            operation,
        } => {
            let (creds, config) = get_creds_and_config(&args.profile).await?;
            let credential_provider = creds.override_and_authenticate(api_key, endpoint)?;

            let client = get_topic_client(credential_provider).await?;
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
                    let subscription = client.subscribe(cache_name, topic).await.map_err(|e| {
                        CliError::new(format!(
                            "the subscription ended without receiving any values: {e:?}"
                        ))
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
            PreviewCommand::Function {
                api_key,
                endpoint,
                operation,
            } => {
                let (creds, config) = get_creds_and_config(&args.profile).await?;
                let credential_provider = creds.override_and_authenticate(api_key, endpoint)?;

                let api_endpoint = credential_provider.cache_http_endpoint().to_string();
                let auth_token = credential_provider.auth_token().to_string();
                let client = get_function_client(credential_provider).await?;

                match operation {
                    momento_cli_opts::FunctionCommand::PutFunction {
                        cache_name,
                        name,
                        wasm_file,
                        id_uploaded_wasm,
                        version_uploaded_wasm,
                        description,
                        environment_variables,
                        metrics_iam_role,
                        disable_metrics,
                        remove_metrics_config,
                    } => {
                        let cache_name = cache_name.unwrap_or(config.cache);
                        let wasm_source = determine_wasm_source(
                            wasm_file,
                            id_uploaded_wasm,
                            version_uploaded_wasm,
                        )?;
                        let metrics_change = determine_function_metrics_config_change(
                            metrics_iam_role,
                            disable_metrics,
                            remove_metrics_config,
                        );
                        commands::functions::function_cli::put_function(
                            client,
                            cache_name,
                            name,
                            wasm_source,
                            description,
                            environment_variables,
                            metrics_change,
                        )
                        .await?
                    }
                    momento_cli_opts::FunctionCommand::PutFunctionConfig {
                        cache_name,
                        function_name,
                        function_id,
                        pin_version,
                        use_latest_version,
                        metrics_iam_role,
                        disable_metrics,
                        remove_metrics_config,
                    } => {
                        let cache_name = cache_name.unwrap_or(config.cache);
                        let new_version =
                            determine_current_function_version(pin_version, use_latest_version);
                        let metrics_change = determine_function_metrics_config_change(
                            metrics_iam_role,
                            disable_metrics,
                            remove_metrics_config,
                        );
                        commands::functions::function_cli::put_function_config(
                            client,
                            cache_name,
                            function_name,
                            function_id,
                            new_version,
                            metrics_change,
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
                        method,
                        data,
                        headers,
                        path,
                    } => {
                        let cache_name = cache_name.unwrap_or(config.cache);
                        commands::functions::function_cli::invoke_function(
                            api_endpoint,
                            auth_token,
                            cache_name,
                            name,
                            method,
                            InvocationOptions {
                                data,
                                headers,
                                path,
                            },
                        )
                        .await?
                    }
                    momento_cli_opts::FunctionCommand::ListFunctions { cache_name } => {
                        let cache_name = cache_name.unwrap_or(config.cache);
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
            PreviewCommand::Pool {
                api_key,
                endpoint,
                operation,
            } => {
                let (creds, _) = get_creds_and_config(&args.profile).await?;
                let credential_provider = creds.override_and_authenticate(api_key, endpoint)?;

                let api_endpoint = credential_provider.cache_http_endpoint().to_string();
                let auth_token = credential_provider.auth_token().to_string();

                match operation {
                    momento_cli_opts::CapacityPoolCommand::Create {
                        name,
                        instance_type,
                        shard_count,
                        replicas_per_shard,
                        capacity_gib,
                        zones,
                        metrics_iam_role,
                        metrics_region,
                        disable_metrics,
                    } => {
                        let provisioning = determine_provisioning(
                            instance_type,
                            shard_count,
                            replicas_per_shard,
                            capacity_gib,
                            zones,
                        )?;
                        let metrics_config = determine_metrics_config(
                            metrics_iam_role,
                            metrics_region,
                            disable_metrics,
                            false,
                        )?;
                        commands::capacity_pool::pool_cli::create_pool(
                            api_endpoint,
                            auth_token,
                            name,
                            provisioning,
                            metrics_config,
                        )
                        .await?
                    }
                    momento_cli_opts::CapacityPoolCommand::GetStatus { name } => {
                        commands::capacity_pool::pool_cli::get_status(
                            api_endpoint,
                            auth_token,
                            name,
                        )
                        .await?
                    }
                    momento_cli_opts::CapacityPoolCommand::Describe { name } => {
                        commands::capacity_pool::pool_cli::describe_pool(
                            api_endpoint,
                            auth_token,
                            name,
                        )
                        .await?
                    }
                    momento_cli_opts::CapacityPoolCommand::Update {
                        name,
                        mode,
                        instance_type,
                        shard_count,
                        replicas_per_shard,
                        capacity_gib,
                        zones,
                        metrics_iam_role,
                        metrics_region,
                        disable_metrics,
                        remove_metrics_config,
                    } => {
                        let existing_metrics_config =
                            commands::capacity_pool::pool_cli::fetch_pool_metrics_config(
                                api_endpoint.clone(),
                                auth_token.clone(),
                                name.clone(),
                            )
                            .await?;
                        let provisioning_update = determine_provisioning_update(
                            mode,
                            instance_type,
                            shard_count,
                            replicas_per_shard,
                            capacity_gib,
                            zones,
                        )?;
                        let metrics_config = determine_metrics_config(
                            metrics_iam_role,
                            metrics_region,
                            disable_metrics,
                            remove_metrics_config,
                        )?
                        .map(|config| config.with_default_region(existing_metrics_config.region()));
                        commands::capacity_pool::pool_cli::update_pool(
                            api_endpoint,
                            auth_token,
                            name,
                            provisioning_update,
                            metrics_config,
                        )
                        .await?
                    }
                    momento_cli_opts::CapacityPoolCommand::Delete { name } => {
                        commands::capacity_pool::pool_cli::delete_pool(
                            api_endpoint,
                            auth_token,
                            name,
                        )
                        .await?
                    }
                    momento_cli_opts::CapacityPoolCommand::List {} => {
                        commands::capacity_pool::pool_cli::list_pools(api_endpoint, auth_token)
                            .await?
                    }
                }
            }
            PreviewCommand::Database {
                api_key,
                endpoint,
                operation,
            } => {
                let (creds, _) = get_creds_and_config(&args.profile).await?;
                let credential_provider = creds.override_and_authenticate(api_key, endpoint)?;

                let api_endpoint = credential_provider.cache_http_endpoint().to_string();
                let valkey_hostname = credential_provider.valkey_hostname().to_string();
                let auth_token = credential_provider.auth_token().to_string();

                match operation {
                    momento_cli_opts::DatabaseCommand::Create {
                        pool_name,
                        name,
                        metrics_iam_role,
                        metrics_region,
                        disable_metrics,
                    } => {
                        let metrics_config = determine_metrics_config(
                            metrics_iam_role,
                            metrics_region,
                            disable_metrics,
                            false,
                        )?;
                        commands::database::database_cli::create_database(
                            api_endpoint,
                            valkey_hostname,
                            auth_token,
                            pool_name,
                            name,
                            metrics_config,
                        )
                        .await?
                    }
                    momento_cli_opts::DatabaseCommand::Describe { name } => {
                        commands::database::database_cli::describe_database(
                            api_endpoint,
                            valkey_hostname,
                            auth_token,
                            name,
                        )
                        .await?
                    }
                    momento_cli_opts::DatabaseCommand::Update {
                        name,
                        metrics_iam_role,
                        metrics_region,
                        disable_metrics,
                        remove_metrics_config,
                    } => {
                        let existing_metrics_config =
                            commands::database::database_cli::fetch_database_metrics_config(
                                api_endpoint.clone(),
                                auth_token.clone(),
                                name.clone(),
                            )
                            .await?;
                        let metrics_config = determine_metrics_config(
                            metrics_iam_role,
                            metrics_region,
                            disable_metrics,
                            remove_metrics_config,
                        )?
                        .map(|config| config.with_default_region(existing_metrics_config.region()));
                        commands::database::database_cli::update_database(
                            api_endpoint,
                            auth_token,
                            name,
                            metrics_config,
                        )
                        .await?
                    }
                    momento_cli_opts::DatabaseCommand::Delete { name } => {
                        commands::database::database_cli::delete_database(
                            api_endpoint,
                            auth_token,
                            name,
                        )
                        .await?
                    }
                    momento_cli_opts::DatabaseCommand::List {} => {
                        commands::database::database_cli::list_databases(
                            api_endpoint,
                            valkey_hostname,
                            auth_token,
                        )
                        .await?
                    }
                }
            }
        },
    }
    Ok(())
}

impl From<MomentoError> for CliError {
    fn from(val: MomentoError) -> Self {
        CliError::new(match &val.inner_error {
            None => format!("{} (SDK {:?})", val.message, val.error_code),
            Some(error_source) => format!(
                "{} (SDK {:?} from: {}{})",
                val.message,
                val.error_code,
                error_source,
                val.details()
                    .map(|details| format!(": {}", details.message))
                    .unwrap_or_default()
            ),
        })
        .with_details(format!("{val:#?}"))
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
        warn!("{e:#?}"); // only in verbose mode (error!() would always output)
        console_info!("{e}");
        exit(1)
    }
}
