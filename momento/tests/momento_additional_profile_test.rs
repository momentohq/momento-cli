mod common;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {

    use crate::common::{
        cache_list_output_contains_cache_predicate, debug_output_momento_config_files,
        get_test_auth_token_from_env_var, get_unique_test_run_id,
        initialize_temp_momento_config_dir,
    };
    use assert_cmd::Command;
    use predicates::prelude::*;

    async fn momento_configure_profiles(test_auth_token: &str, profile_name: &str) {
        // first create a default profile with a bunk auth token to make sure that none of the other
        // commands use the default profile
        let mut cmd1 = Command::cargo_bin("momento").unwrap();
        let bogus_auth_token = "eyJhbGciOiJIUzUxMiJ9.eyJzdWIiOiJmb29AdGVzdC5ub3RhcmVhbGRvbWFpbiIsImNwIjoiY29udHJvbC1wbGFuZS1lbmRwb2ludC50ZXN0Lm5vdGFyZWFsZG9tYWluIiwiYyI6ImNhY2hlLWVuZHBvaW50LnRlc3Qubm90YXJlYWxkb21haW4ifQo.rtxfu4miBHQ1uptWJ2x3UiAwwJYcMeYIkkpXxUno_wIavg4h6YJStcbxk32NDBbmJkJS7mUw6MsvJNWaxfdPOw";
        cmd1.args(["configure"])
            .write_stdin(bogus_auth_token)
            .assert()
            .failure()
            .stderr(
                predicate::str::is_match("error trying to connect: dns error:")
                    .expect("Unable to create dns error predicate"),
            );

        // now create the additional profile, which we will use for all of our tests.
        let mut cmd2 = Command::cargo_bin("momento").unwrap();
        cmd2.args(["configure", "--profile", profile_name])
            .write_stdin(test_auth_token)
            .assert()
            .success();
    }

    async fn momento_cache_create_with_profile(profile_name: &str, cache_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args([
            "cache",
            "create",
            "--name",
            cache_name,
            "--profile",
            profile_name,
        ])
        .assert()
        .success();
    }

    async fn momento_cache_set_with_profile(profile_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args([
            "cache",
            "set",
            "--key",
            "key",
            "--value",
            "value",
            "--profile",
            profile_name,
        ])
        .assert()
        .success();
    }

    async fn momento_cache_get_with_profile(profile_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "get", "--key", "key", "--profile", profile_name])
            .assert()
            .stdout("value\n");
    }

    async fn momento_cache_list_with_profile(profile_name: &str, cache_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "list", "--profile", profile_name])
            .assert()
            .stdout(cache_list_output_contains_cache_predicate(cache_name));
    }

    async fn momento_cache_delete_with_profile(profile_name: &str, cache_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args([
            "cache",
            "delete",
            "--name",
            cache_name,
            "--profile",
            profile_name,
        ])
        .assert()
        .success();
    }

    async fn test_profile_allowed_in_any_position(profile_name: &str) {
        let profile_permutations = vec![
            // cache subcommand
            vec!["cache", "get", "--key", "key", "--profile", profile_name],
            vec!["cache", "get", "--profile", profile_name, "--key", "key"],
            vec!["cache", "--profile", profile_name, "get", "--key", "key"],
            vec!["--profile", profile_name, "cache", "get", "--key", "key"],
            // configure subcommand
            vec!["configure", "--profile", profile_name],
            vec!["--profile", profile_name, "configure"],
            // signing-key subcommand
            vec!["signing-key", "list", "--profile", profile_name],
            vec!["signing-key", "--profile", profile_name, "list"],
            vec!["--profile", profile_name, "signing-key", "list"],
            // account subcommand
            vec!["account", "signup", "--profile", profile_name, "help"],
            vec!["account", "--profile", profile_name, "signup", "help"],
            vec!["--profile", profile_name, "account", "signup", "help"],
        ];
        for command_line_args in profile_permutations {
            let mut cmd = Command::cargo_bin("momento").unwrap();
            // Exit status 2 indicates a CLI parsing error
            cmd.args(command_line_args).assert().code(predicate::ne(2));
        }
    }

    #[tokio::test]
    async fn momento_additional_profile() {
        let test_auth_token = get_test_auth_token_from_env_var();
        let test_run_id = get_unique_test_run_id();
        let test_momento_home_dir = initialize_temp_momento_config_dir(&test_run_id);

        momento_configure_profiles(&test_auth_token, &test_run_id).await;
        momento_cache_create_with_profile(&test_run_id, &test_run_id).await;
        momento_cache_set_with_profile(&test_run_id).await;
        momento_cache_get_with_profile(&test_run_id).await;
        momento_cache_list_with_profile(&test_run_id, &test_run_id).await;
        momento_cache_delete_with_profile(&test_run_id, &test_run_id).await;
        test_profile_allowed_in_any_position(&test_run_id).await;

        debug_output_momento_config_files(test_momento_home_dir.path());

        test_momento_home_dir
            .close()
            .expect("Unable to close momento config temp dir");
    }
}
