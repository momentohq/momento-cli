#!/usr/bin/env bash

set -e
set -x
set -o pipefail

if [ "$TEST_AUTH_TOKEN" == "" ]
then
  echo "Missing required env var TEST_AUTH_TOKEN"
  exit 1
fi

cargo test --test momento_default_profile_test
cargo test --test momento_additional_profile_test
