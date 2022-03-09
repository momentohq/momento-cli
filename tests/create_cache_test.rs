#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    #[tokio::test]
    async fn momento_cache_create_default_profile() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "create", "--name", &test_cache_default])
            .assert()
            .success();
    }

    #[tokio::test]
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
}
