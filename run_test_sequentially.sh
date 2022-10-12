#!/usr/bin/env bash

set -e
set -x
set -o pipefail

if [ "$TEST_CACHE_DEFAULT" == "" ]
then
  echo "Missing required env var TEST_CACHE_DEFAULT"
  exit 1
else 
    export TEST_CACHE_DEFAULT=$TEST_CACHE_DEFAULT
fi

if [ "$TEST_CACHE_WITH_PROFILE" == "" ]
then
  echo "Missing required env var TEST_CACHE_WITH_PROFILE"
  exit 1
else
    export TEST_CACHE_WITH_PROFILE=$TEST_CACHE_WITH_PROFILE
fi

if [ "$TEST_PROFILE" == "" ] 
then
  echo "Missing required env var TEST_PROFILE"
  exit 1
else 
    export TEST_PROFILE=$TEST_PROFILE
fi

cargo test --test momento_default_profile
cargo test --test momento_additional_profile
