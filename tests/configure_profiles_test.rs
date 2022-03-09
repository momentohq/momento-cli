#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn configure_momento_named_profile() {
        // disabling these tests for now since they make an api call to create cache
        // let mut cmd = Command::cargo_bin("momento").unwrap();
        // cmd.args(&["configure", "--profile", "TEST_PROFILE"])
        //     .write_stdin("token\ncache\n999")
        //     .assert()
        //     .success();
    }
}
