#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    #[tokio::test]
    async fn momento_cache_set_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "set", "--key", "key", "--value", "value"])
            .assert()
            .success();
    }

    #[tokio::test]
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
}
