# Momento CLI

Japanese: [日本語](README.ja.md)






## Quick Start
```
# Install 
brew tap momentohq/tap
brew install momento-cli

# Sign Up [available regions are us-west-2, us-east-1, ap-northeast-1, default is us-west-2]
momento account signup --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>

# Configure your account with teh credentials in your email
momento configure

# Make a cache
momento cache create --name example-cache

# Set and Get values from your cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache

```

## Upgrading
```
brew update momento-cli
brew upgrade momento-cli
```

## Sign up 


**NOTE:** If you run into errors during signup, please ensure you have upgraded to the [latest version](https://github.com/momentohq/momento-cli/releases/latest) of our CLI.

```
# default region is us-west-2
momento account signup --email <TYPE_YOUR_EMAIL_HERE>

# (optional) view help to see all available regions, and sign up for a specific region
momento account signup --help
momento account signup --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>

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
