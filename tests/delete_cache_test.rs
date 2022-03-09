#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    #[tokio::test]
    async fn momento_cache_delete_default() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "delete", "--name", &test_cache_default])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_cache_delete_profile() {
        let test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["cache", "delete", "--name", &test_cache_with_profile])
            .assert()
            .success();
    }
}
