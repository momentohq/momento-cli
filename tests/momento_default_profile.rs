#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::predicate;
    use std::str;

    async fn momento_cache_create_default_profile() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        let output = Command::cargo_bin("momento")
            .unwrap()
            .args(["cache", "list"])
            .output()
            .unwrap()
            .stdout;
        if !output.is_empty() {
            let string_output = str::from_utf8(&output).unwrap();
            let v: Vec<&str> = string_output.split('\n').collect();
            for cache in v.iter() {
                if !cache.is_empty() {
                    Command::cargo_bin("momento")
                        .unwrap()
                        .args(["cache", "delete", "--name", cache])
                        .unwrap();
                }
            }
        }
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "create", "--name", &test_cache_default])
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

    async fn momento_cache_list_default_profile() {
        let mut test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        test_cache_default.push('\n');
        let mut cmd = Command::cargo_bin("momento").unwrap();
        let output = cmd.args(["cache", "list"]).output().unwrap().stdout;
        let string_output = str::from_utf8(&output).unwrap();
        if !string_output.split('\n').any(|x| x == &*test_cache_with_profile) {
            // Exit status 3 indicates cache list operation didn't include test_cache_default in the returned list.
            cmd.args(["cache", "list"]).assert().code(predicate::ne(3));
        }
    }

    async fn momento_cache_delete_default_profile() {
        let test_cache_default = std::env::var("TEST_CACHE_DEFAULT").unwrap();
        let mut cmd = Command::cargo_bin("momento").unwrap();
        cmd.args(["cache", "delete", "--name", &test_cache_default])
            .assert()
            .success();
    }

    #[tokio::test]
    async fn momento_default_profile() {
        momento_cache_create_default_profile().await;
        momento_cache_set_default_profile().await;
        momento_cache_get_default_profile().await;
        momento_cache_list_default_profile().await;
        momento_cache_delete_default_profile().await;
    }
}
