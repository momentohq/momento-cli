#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn configure_momento_named_profile() {
        // disabling these tests for now since this behavior will change soon
        // Related task: https://github.com/momentohq/momento-cli/issues/58
        // let mut cmd = Command::cargo_bin("momento").unwrap();
        // cmd.args(&["configure", "--profile", "TEST_PROFILE"])
        //     .write_stdin("token\ncache\n999")
        //     .assert()
        //     .success();
    }
}
