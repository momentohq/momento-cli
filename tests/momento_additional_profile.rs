#[cfg(test)]
mod tests {

    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::str;

    async fn momento_cache_create_with_profile() {
        let test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args([
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
        cmd.args([
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
        cmd.args(["cache", "get", "--key", "key", "--profile", &test_profile])
            .assert()
            .stdout("value\n");
    }

    async fn momento_cache_list_with_profile() {
        let mut test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        test_cache_with_profile.push('\n');
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        let output = cmd
            .args(["cache", "list", "--profile", &test_profile])
            .output()
            .unwrap()
            .stdout;
        let string_output = str::from_utf8(&output).unwrap();
        assert!(string_output
            .split('\n')
            .any(|x| x == test_cache_with_profile));
    }

    async fn momento_cache_delete_with_profile() {
        let test_cache_with_profile = std::env::var("TEST_CACHE_WITH_PROFILE").unwrap();
        let test_profile = std::env::var("TEST_PROFILE").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args([
            "cache",
            "delete",
            "--name",
            &test_cache_with_profile,
            "--profile",
            &test_profile,
        ])
        .assert()
        .success();
    }

    async fn test_profile_allowed_in_any_position() {
        let test_profile = std::env::var("TEST_PROFILE").unwrap();

        let profile_permutations = vec![
            // cache subcommand
            vec!["cache", "get", "--key", "key", "--profile", &test_profile],
            vec!["cache", "get", "--profile", &test_profile, "--key", "key"],
            vec!["cache", "--profile", &test_profile, "get", "--key", "key"],
            vec!["--profile", &test_profile, "cache", "get", "--key", "key"],
            // configure subcommand
            vec!["configure", "--profile", &test_profile],
            vec!["--profile", &test_profile, "configure"],
            // account subcommand
            vec!["account", "list-signing-keys", "--profile", &test_profile],
            vec!["account", "--profile", &test_profile, "list-signing-keys"],
            vec!["--profile", &test_profile, "account", "list-signing-keys"],
        ];
        for command_line_args in profile_permutations {
            let mut cmd = Command::cargo_bin("momento").unwrap();
            // Exit status 2 indicates a CLI parsing error
            cmd.args(command_line_args).assert().code(predicate::ne(2));
        }
    }

    #[tokio::test]
    async fn momento_additional_profile() {
        momento_cache_create_with_profile().await;
        momento_cache_set_with_profile().await;
        momento_cache_get_with_profile().await;
        momento_cache_list_with_profile().await;
        momento_cache_delete_with_profile().await;
        test_profile_allowed_in_any_position().await;
    }
}
