use predicates::str::RegexPredicate;
use std::fs::File;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tempdir::TempDir;
use uuid::Uuid;

const TEST_DEBUG_OUTPUT_ENABLED: bool = true;

pub fn get_unique_test_run_id() -> String {
    let test_run_uuid = Uuid::new_v4();
    format!("momento-cli-{test_run_uuid}")
}

pub fn get_test_auth_token_from_env_var() -> String {
    std::env::var("TEST_AUTH_TOKEN").expect("Missing required env var TEST_AUTH_TOKEN")
}

pub fn initialize_temp_momento_config_dir(test_run_id: &str) -> TempDir {
    let test_momento_config_dir = TempDir::new(test_run_id).expect("Unable to create temp dir");
    let test_momento_config_dir_path = fs::canonicalize(test_momento_config_dir.path())
        .expect("Unable to canonicalize path")
        .into_os_string()
        .into_string()
        .expect("Unable to convert canonical path to string");
    if TEST_DEBUG_OUTPUT_ENABLED {
        println!("Initialized temporary momento config dir {test_momento_config_dir_path}");
    }
    std::env::set_var("MOMENTO_CONFIG_DIR", &test_momento_config_dir_path);
    test_momento_config_dir
}

pub fn cache_list_output_contains_cache_predicate(cache_name: &str) -> RegexPredicate {
    predicates::str::is_match(format!("(?m)^{cache_name}$"))
        .expect("Unable to create predicate from cache name")
}

pub fn debug_output_momento_config_files(config_dir: &Path) {
    if !TEST_DEBUG_OUTPUT_ENABLED {
        return;
    }

    println!();
    let final_config_path = config_dir.join("config");
    debug_output_file(&final_config_path);

    println!();
    let final_creds_path = config_dir.join("credentials");
    debug_output_file(&final_creds_path);
}

fn debug_output_file(file_path: &PathBuf) {
    println!("Momento config / creds file ({file_path:?}) contents:");
    let lines = io::BufReader::new(
        File::open(file_path).unwrap_or_else(|e| panic!("Unable to open file: {file_path:?}: {e:?}")),
    )
    .lines();
    for l in lines {
        let line = l.expect("Unable to read line from file");
        if line.starts_with("token") {
            println!("token=<REDACTED>");
        } else {
            println!("{line}");
        }
    }
}
