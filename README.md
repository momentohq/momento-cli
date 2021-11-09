## Prerequisites
- MacOSX or Linux
- [Homebrew](https://brew.sh/)

## Installation
```bash
brew tap momentohq/tap
brew install momento-cli
```

## Configure
```
momento configure
```
This will prompt you for your momento auth token, and save it to be reused.

## Using Momento CLI
```
momento cache create --name example-cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache
```
