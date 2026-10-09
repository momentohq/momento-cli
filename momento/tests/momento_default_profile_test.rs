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
    use predicates::Predicate;
    use std::str;

    async fn configure_momento_default_profile(test_auth_token: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["configure", "--disposable-token"])
            .write_stdin(test_auth_token)
            .assert()
            .success();
    }

    async fn momento_cache_create_default_profile(cache_name: &str) {
        let output = Command::cargo_bin("momento")
            .unwrap()
            .args(["cache", "list"])
            .output()
            .unwrap()
            .stdout;
        let string_output =
            str::from_utf8(&output).expect("Unable to convert cache list output to a utf8 string");
        if cache_list_output_contains_cache_predicate(cache_name).eval(string_output) {
            let v: Vec<&str> = string_output.split('\n').collect();
            for cache in v.iter() {
                if !cache.is_empty() {
                    Command::cargo_bin("momento")
                        .unwrap()
                        .args(["cache", "delete", "--name", cache_name])
                        .unwrap();
                }
            }
        }
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "create", "--name", cache_name])
            .assert()
            .success();
    }

    async fn momento_cache_set_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "set", "--key", "key", "--value", "value"])
            .assert()
            .success();
    }

    async fn momento_cache_get_default_profile() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "get", "--key", "key"])
            .assert()
            .stdout("value\n");
    }

    async fn momento_cache_set_default_profile_positional_args() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "set", "positional-key", "positional-value"])
            .assert()
            .success();
    }

    async fn momento_cache_get_default_profile_positional_args() {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "get", "positional-key"])
            .assert()
            .stdout("positional-value\n");
    }

    async fn momento_cache_list_default_profile(cache_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "list"])
            .assert()
            .stdout(cache_list_output_contains_cache_predicate(cache_name));
    }

    async fn momento_cache_delete_default_profile(cache_name: &str) {
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "delete", "--name", cache_name])
            .assert()
            .success();
    }

    async fn momento_cache_create_delete_default_profile_positional_args(cache_name: &str) {
        let mut cmd1 = Command::cargo_bin("momento").unwrap();
        cmd1.args(["cache", "create", cache_name])
            .assert()
            .success();

        let mut cmd2 = Command::cargo_bin("momento").unwrap();
        cmd2.args(["cache", "delete", cache_name])
            .assert()
            .success();
    }

    async fn momento_cache_create_delete_default_cache_keyword_arg(cache_name: &str) {
        let mut cmd1 = Command::cargo_bin("momento").unwrap();
        cmd1.args(["cache", "create", "--cache", cache_name])
            .assert()
            .success();

        let mut cmd2 = Command::cargo_bin("momento").unwrap();
        cmd2.args(["cache", "delete", "--cache", cache_name])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_default_profile() {
        let test_auth_token = get_test_auth_token_from_env_var();
        let test_run_id = get_unique_test_run_id();
        let test_momento_home_dir = initialize_temp_momento_config_dir(&test_run_id);

        configure_momento_default_profile(&test_auth_token).await;
        momento_cache_create_default_profile(&test_run_id).await;
        momento_cache_set_default_profile().await;
        momento_cache_get_default_profile().await;
        momento_cache_set_default_profile_positional_args().await;
        momento_cache_get_default_profile_positional_args().await;
        momento_cache_list_default_profile(&test_run_id).await;
        momento_cache_delete_default_profile(&test_run_id).await;
        momento_cache_create_delete_default_profile_positional_args(&test_run_id).await;
        momento_cache_create_delete_default_cache_keyword_arg(&test_run_id).await;

        debug_output_momento_config_files(test_momento_home_dir.path());

        test_momento_home_dir
            .close()
            .expect("Unable to close momento config temp dir");
    }
}
