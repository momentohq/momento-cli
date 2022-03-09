#[cfg(test)]
mod tests {

    use assert_cmd::Command;
    #[tokio::test]
    async fn momento_cache_get_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "get", "--key", "key"])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_cache_get_with_profile() {
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "get", "--key", "key", "--profile", &test_profile])
            .assert()
            .success();
    }
}
