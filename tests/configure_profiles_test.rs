#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    async fn configure_momento_default_profile() {
        let test_auth_token = std::env::var("TEST_AUTH_TOKEN_DEFAULT")
            .expect("Missing required env var TEST_AUTH_TOKEN_DEFAULT");
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["configure"])
            .write_stdin(test_auth_token)
            .assert()
            .success();
    }

    async fn momento_cache_set_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "set", "--key", "key", "--value", "value"])
            .assert()
            .success();
    }

    async fn momento_cache_get_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "get", "--key", "key"])
            .assert()
            .stdout("value\n");
    }

    async fn momento_cache_delete_default_profile() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT")
            .expect("Missing required env var TEST_CACHE_DEFAULT");
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "delete", "--name", &test_cache_default])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_configured_default_profile() {
        configure_momento_default_profile().await;
        momento_cache_set_default_profile().await;
        momento_cache_get_default_profile().await;
        momento_cache_delete_default_profile().await;
    }
}
