#[cfg(test)]
mod tests {
    use std::path::Path;

    use assert_cmd::Command;

    #[tokio::test]
    async fn configure_momento_default_profile() {
        if Path::new("~/.momento").exists() {
            panic!("These integration tests test reading profiles from disk, and create a ~/.momento directory to test this.
            To avoid overriding existing profiles, this error has been thrown.
            If you a want to run these tests, run 'mv ~/.momento ~/.momento.bac' to save the current profiles.
            After these tests complete run 'mv ~/.momento.bac ~/.momento' to restore the profiles");
        };
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["configure"])
            .write_stdin("token\ncache\n999")
            .assert()
            .success();
    }
}
