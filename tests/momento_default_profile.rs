#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    async fn momento_cache_create_default_profile() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        Command::cargo_bin("momento")
            .unwrap()
            .args(&["cache", "delete", "--name", &test_cache_default])
            .unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "create", "--name", &test_cache_default])
            .assert()
            .success();
    }

    async fn momento_cache_set_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "set", "--key", "key", "--value", "value"])
            .assert()
            .success();
    }

    async fn momento_cache_get_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "get", "--key", "key"])
            .assert()
            .stdout("value\n");
    }

    async fn momento_cache_list_default_profile() {
        let mut test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        test_cache_default.push('\n');
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "list"])
            .assert()
            .stdout(test_cache_default);
    }

    async fn momento_cache_delete_default_profile() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "delete", "--name", &test_cache_default])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_default_profile() {
        momento_cache_create_default_profile().await;
        momento_cache_set_default_profile().await;
        momento_cache_get_default_profile().await;
        momento_cache_list_default_profile().await;
        momento_cache_delete_default_profile().await;
    }
}
