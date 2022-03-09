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
```
# default region is us-west-2
momento account signup --email <TYPE_YOUR_EMAIL_HERE>

# (optional) view help to see all available regions, and sign up for a specific region
momento account signup --help
momento account signup --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>
```
Upon signing up, Momento sends a token to the email provided. This token uniquely identifies cache interactions. The token should be treated like a sensitive password and all essential care must be taken to ensure its secrecy. We recommend that you store this token in a secret vault like AWS Secrets Manager.

## Configure

```
momento configure
```

This will prompt you for your Momento Auth Token, default cache name, default TTL, and save them to be reused as a part of your `default` profile.

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
