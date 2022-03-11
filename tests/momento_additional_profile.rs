#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    async fn momento_cache_create_with_profile() {
        let test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&[
            "cache",
            "create",
            "--name",
            &test_cache_with_profile,
            "--profile",
            &test_profile,
        ])
        .assert()
        .success();
    }

    async fn momento_cache_set_with_profile() {
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&[
            "cache",
            "set",
            "--key",
            "key",
            "--value",
            "value",
            "--profile",
            &test_profile,
        ])
        .assert()
        .success();
    }

    async fn momento_cache_get_with_profile() {
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "get", "--key", "key", "--profile", &test_profile])
            .assert()
            .stdout("value\n");
    }

    async fn momento_cache_list_with_profile() {
        let mut test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        test_cache_with_profile.push_str("\n");
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "list", "--profile", &test_profile])
            .assert()
            .stdout(test_cache_with_profile);
    }

    async fn momento_cache_delete_profile() {
        let test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "delete", "--name", &test_cache_with_profile])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_additional_profile() {
        momento_cache_create_with_profile().await;
        momento_cache_set_with_profile().await;
        momento_cache_get_with_profile().await;
        momento_cache_list_with_profile().await;
    }
}
