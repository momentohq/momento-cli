# Momento CLI

_Read this in other languages_: [日本語](README.ja.md)
<br >

## Prerequisites

- MacOS or Linux
- [Homebrew](https://brew.sh/)

## Install

```
brew tap momentohq/tap
brew install momento-cli
```

## Sign up

Currently supported AWS regions: `us-east-1` or `us-west-2`

```
momento account signup --region <TYPE_DESIRED_REGION> --email <TYPE_YOUR_EMAIL_HERE>
```

This generates an access token and sends it to the email provided. This token uniquely identifies cache interactions. The token should be treated like a sensitive password and all essential care must be taken to ensure its secrecy. We recommend that you store this token in a secret vault like AWS Secrets Manager.

## Configure

```
momento configure
```

This will prompt you for your Momento Auth Token, and save it to be reused.

## Use CLI

```
momento cache create --name example-cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache
```

## Use Momento in Your Project

Check out our [SDKs](https://github.com/momentohq/client-sdk-examples) to integrate Momento into your project!

## Contributing

If you would like to contribute to the Momento Cli, please read out [Contributing Guide](./CONTRIBUTING.md)
