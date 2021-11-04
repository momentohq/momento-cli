#!/bin/bash

function log() {
  local msg=$1
  >&2 echo "$msg"
}

function usage() {
  log "
This script updates the Cargo.toml file to match the semver release version
Usage:
$0 <SEMVER_VERSION>
Example:
$0 1.0.3
  "
  exit 1
}

if [ "$#" -ne 1 ]; then
    log "Expected 1 positional argument: <SEMVER_VERSION>"
    log ""
    usage
fi

log "updating Cargo.toml to version $1"

pip3 install toml
python3 update_cargo_toml.py $1
