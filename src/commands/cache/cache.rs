use log::info;

pub async fn create_cache(cache_name: String, auth_token: String) {
    info!("create cache called");
}

pub async fn delete_cache(cache_name: String, auth_token: String) {
    info!("delete cache called");
}

pub async fn set(
    cache_name: String,
    auth_token: String,
    key: String,
    value: String,
    ttl_seconds: u32,
) {
    info!("cache set called");
}

pub async fn get(cache_name: String, auth_token: String, key: String) {
    info!("cache get called");
}
