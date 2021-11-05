Momento Client CLI

## Prerequisite
1. [Homebrew](https://brew.sh/)

## Installation
1. brew tap momentohq/tap
1. brew install momento-cli

## Instructions
1. after installation, run the command `momento configure`. This will prompt you for
your momento auth token, and save it to be reused.

## Using Momento Cache
1. momento cache create --name example-cache
1. momento cache set --key key --value value --ttl 100 --name example-cache
1. momento cache get --key key --name example-cache
1. momento cache delete --name example-cache
