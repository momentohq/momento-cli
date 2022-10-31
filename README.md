# Momento CLI

Japanese: [日本語](README.ja.md)
Portuguese: [Português](README.pt.md)

## Quick Start

Please refer to the installation instructions for Linux [here](https://github.com/momentohq/momento-cli/blob/main/README.md#linux) and Windows [here](https://github.com/momentohq/momento-cli/blob/main/README.md#windows).

```
# Install
brew tap momentohq/tap
brew install momento-cli

# Sign Up

## AWS [available regions are us-west-2, us-east-1, ap-northeast-1]
momento account signup aws --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>

## GCP [available regions are us-east1, asia-northeast1]
momento account signup gcp --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>


# Configure your account with the credentials in your email, plus default cache name and TTL
momento configure --quick

# Make a cache
momento cache create --name example-cache

# Set and Get values from your cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache

```

## Installation

### Linux

1. Download the latest linux tar.gz file from [https://github.com/momentohq/momento-cli/releases/latest](https://github.com/momentohq/momento-cli/releases/latest)
2. Unzip the file: `tar -xvf momento-cli-X.X.X.linux_x86_64.tar.gz`
3. Move `./momento` to your execution path.

### Windows

1. Download the latest windows zip file from [https://github.com/momentohq/momento-cli/releases/latest](https://github.com/momentohq/momento-cli/releases/latest)
2. Unzip the `momento-cli-X.X.X.windows_x86_64.zip` file
3. Run the unzipped .exe file

## Upgrading

```
brew upgrade momento-cli
```

## Sign up

**NOTE:** If you run into errors during signup, please ensure you have upgraded to the [latest version](https://github.com/momentohq/momento-cli/releases/latest) of our CLI.

### Momento on AWS
```
# View help to see all available regions, and sign up for a specific region
momento account signup aws --help
momento account signup aws --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>

# Configure CLI
momento configure
```

### Momento on GCP
```
# View help to see all available regions, and sign up for a specific region
momento account signup gcp --help
momento account signup gcp --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>

# Configure CLI
momento configure
```

Upon signing up, Momento sends a token to the email provided. This token uniquely identifies cache interactions. The token should be treated like a sensitive password and all essential care must be taken to ensure its secrecy. We recommend that you store this token in a secret vault like AWS Secrets Manager.

## Configure

### First time configuration

```
# default profile name is default
momento configure
```

This will prompt you for your Momento Auth Token, default cache name, default TTL, and save them to be reused as a part of your `default` profile.

```
momento configure --profile new-profile
```

This will prompt you the same as above and save them to be reused as a part of your `new-profile` profile.

<br>

### Update existing configuration

To update your desired profile, use the same command as above.

## Use CLI

```
# use default profile
momento cache create --name example-cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache
```

You can also specify your desired profile.

```
# use new-profile
momento cache create --name example-cache --profile new-profile
momento cache set --key key --value value --ttl 100 --name example-cache --profile new-profile
momento cache get --key key --name example-cache --profile new-profile
```

## Use Momento in Your Project

Check out our [SDKs](https://github.com/momentohq/client-sdk-examples) to integrate Momento into your project!

## Contributing

If you would like to contribute to the Momento Cli, please read out [Contributing Guide](./CONTRIBUTING.md)
