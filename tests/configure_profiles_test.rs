#[cfg(test)]
mod tests {

    use assert_cmd::Command;

    #[tokio::test]
    async fn configure_momento_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(&["configure"])
            .write_stdin("token\ncache\n999")
            .assert()
            .success();
    }
}
