#[cfg(test)]
mod tests {
    use std::path::Path;

    use assert_cmd::Command;
    use home::home_dir;

    #[tokio::test]
    async fn configure_momento_named_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["configure", "--profile", "TEST_PROFILE"])
            .write_stdin("token\ncache\n999")
            .assert()
            .success();
    }
}
