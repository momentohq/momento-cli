## Prerequisites
- MacOS or Linux
- [Homebrew](https://brew.sh/)

## Install
```bash
brew tap momentohq/tap
brew install momento-cli
```

## Configure
```
momento configure
```
This will prompt you for your Momento Auth Token, and save it to be reused. Need a token? Send us an email at support@momentohq.com.

## Use CLI
```
momento cache create --name example-cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache
```

## Use Momento in Your Project
Check out our [SDKs](https://github.com/momentohq/client-sdk-examples) to integrate Momento into your project!
