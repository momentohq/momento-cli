#!/usr/bin/env bash

set -e
set -x
set -o pipefail

if [ "$TEST_CACHE_DEFAULT" == "" ]
then
  echo "Missing required env var $TEST_CACHE_DEFAULT"
  exit 1
else 
    export TEST_CACHE_DEFAULT=$TEST_CACHE_DEFAULT
fi

if [ "$TEST_CACHE_WITH_PROFILE" == "" ]
then
  echo "Missing required env var $TEST_CACHE_WITH_PROFILE"
  exit 1
else
    export TEST_CACHE_WITH_PROFILE=$TEST_CACHE_WITH_PROFILE
fi

if [ "$TEST_PROFILE" == "" ] 
then
  echo "Missing required env var $TEST_PROFILE"
  exit 1
else 
    export TEST_PROFILE=$TEST_PROFILE
fi

cargo test --test create_cache_test && cargo test --test set_test && cargo test --test get_test && cargo test --test list_cache_test && cargo test --test delete_cache_test
