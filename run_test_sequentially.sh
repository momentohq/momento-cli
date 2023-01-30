#!/usr/bin/env bash

set -e
set -x
set -o pipefail

features=$1

if [ "$features" != "" ]
then
  features="--features $1"
fi

if [ "$TEST_AUTH_TOKEN" == "" ]
then
  echo "Missing required env var TEST_AUTH_TOKEN"
  exit 1
fi

cargo test --test momento_default_profile $features
cargo test --test momento_additional_profile $features
