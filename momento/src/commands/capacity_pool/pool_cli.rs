use super::utils::{
    build_pool_update_body, call_pool_api, call_pool_delete_api, call_pool_families_api,
    call_pool_instance_types_api, call_pool_list_api, CapacityPool, CapacityPoolProvisioning,
    CapacityPoolProvisioningMode, CapacityPoolProvisioningUpdate, CapacityPoolResponse,
};
use crate::commands::capacity_pool::utils::{
    DiscoverFamiliesResponse, DiscoverInstanceTypesResponse, ListCapacityPoolsResponse,
};
use crate::commands::utils::CustomerMetricsConfig;
use crate::commands::utils::{
    unexpectedly_empty_success_from_api,
    MomentoHttpResponse::{Parsed, Unparseable},
};
use crate::{error::CliError, utils::console::console_data};

use http::Method;
use serde_json;

pub async fn create_pool(
    endpoint: String,
    auth_token: String,
    name: String,
    provisioning: CapacityPoolProvisioning,
    metrics_config: Option<CustomerMetricsConfig>,
) -> Result<(), CliError> {
    let data = serde_json::to_value(CapacityPool {
        provisioning,
        metrics_config,
    })?;
    match call_pool_api(Method::POST, endpoint, auth_token, name, Some(data)).await? {
        Parsed(pool) => {
            console_data!("Creating capacity pool!\n\n{pool}");
        }
        Unparseable(response_text) => {
            console_data!("Attempting to create capacity pool!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn get_status(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    match call_pool_api(Method::GET, endpoint, auth_token, name, None).await? {
        Parsed(pool) => {
            console_data!("{}", pool.status);
        }
        Unparseable(response_text) => {
            if response_text.is_empty() {
                return Err(unexpectedly_empty_success_from_api());
            } else {
                console_data!("{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn describe_pool(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    match call_pool_api(Method::GET, endpoint, auth_token, name, None).await? {
        Parsed(pool) => {
            console_data!("Your capacity pool:\n\n{pool}");
        }
        Unparseable(response_text) => {
            if response_text.is_empty() {
                return Err(unexpectedly_empty_success_from_api());
            } else {
                console_data!("Couldn't parse capacity pool:\n\n{response_text}");
            }
        }
    };
    Ok(())
}

/// The pool's current details, for filling in what an update leaves unspecified.
pub async fn fetch_pool(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<CapacityPoolResponse, CliError> {
    match call_pool_api(Method::GET, endpoint, auth_token, name.clone(), None).await? {
        Parsed(pool) => Ok(pool),
        Unparseable(response_text) => Err(CliError::new(format!(
            "Couldn't read the current details of capacity pool {name}"
        ))
        .with_details(response_text)),
    }
}

pub async fn update_pool(
    endpoint: String,
    auth_token: String,
    name: String,
    provisioning_mode: CapacityPoolProvisioningMode,
    provisioning_update: Option<CapacityPoolProvisioningUpdate>,
    metrics_config: Option<CustomerMetricsConfig>,
) -> Result<(), CliError> {
    let data = build_pool_update_body(
        provisioning_mode,
        provisioning_update.clone(),
        metrics_config,
    )?;
    match call_pool_api(Method::PATCH, endpoint, auth_token, name, Some(data)).await? {
        Parsed(mut pool) => {
            pool.hide_lagging_target(provisioning_update);
            console_data!("Updating capacity pool!\n\n{pool}");
        }
        Unparseable(response_text) => {
            console_data!("Attempting to update capacity pool!");
            if !response_text.is_empty() {
                console_data!("\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn delete_pool(
    endpoint: String,
    auth_token: String,
    name: String,
) -> Result<(), CliError> {
    let response_text = call_pool_delete_api(endpoint, auth_token, name.clone()).await?;
    console_data!("Deleting capacity pool {name}!");
    if !response_text.is_empty() {
        console_data!("\n\n{response_text}");
    }
    Ok(())
}

pub async fn list_pools(endpoint: String, auth_token: String) -> Result<(), CliError> {
    let response = call_pool_list_api(endpoint, auth_token).await?;
    match response {
        Parsed(ListCapacityPoolsResponse {
            capacity_pools: pools_list,
        }) => {
            if pools_list.is_empty() {
                console_data!("No capacity pools found");
            } else {
                console_data!("Capacity pools:");
                for pool in pools_list.iter() {
                    console_data!("\n{pool}");
                }
            }
        }
        Unparseable(response_text) => {
            if response_text.is_empty() {
                return Err(unexpectedly_empty_success_from_api());
            } else {
                console_data!("Couldn't parse list of capacity pools:\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn discover_families(api_endpoint: String, auth_token: String) -> Result<(), CliError> {
    let response = call_pool_families_api(api_endpoint, auth_token).await?;
    match response {
        Parsed(DiscoverFamiliesResponse { families }) => {
            if families.is_empty() {
                console_data!("No capacity families are available to you");
            } else {
                console_data!("For flex-mode capacity pools, you can use these capacity families:");
                for family in families.iter() {
                    console_data!("\n{family}");
                }
            }
        }
        Unparseable(response_text) => {
            if response_text.is_empty() {
                return Err(unexpectedly_empty_success_from_api());
            } else {
                console_data!("Couldn't parse list of capacity families:\n\n{response_text}");
            }
        }
    };
    Ok(())
}

pub async fn discover_instance_types(
    api_endpoint: String,
    auth_token: String,
) -> Result<(), CliError> {
    let response = call_pool_instance_types_api(api_endpoint, auth_token).await?;
    match response {
        Parsed(DiscoverInstanceTypesResponse { instance_types }) => {
            if instance_types.is_empty() {
                console_data!("No instance types are available to you");
            } else {
                console_data!(
                    "For explicit-mode capacity pools, you can use these instance types:\n\n{}",
                    instance_types.join("\n")
                );
            }
        }
        Unparseable(response_text) => {
            if response_text.is_empty() {
                return Err(unexpectedly_empty_success_from_api());
            } else {
                console_data!("Couldn't parse list of instance types:\n\n{response_text}");
            }
        }
    };
    Ok(())
}
