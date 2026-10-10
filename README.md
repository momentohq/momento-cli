# Momento CLI

Japanese: [日本語](README.ja.md)
Portuguese: [Português](README.pt.md)

Command-line tool for managing Momento.  Supports the following:

* Create a Momento account
* Momento Cache:
  * Create, list, and delete capacity pools
  * Create, list, and delete Valkey databases
* Momento API keys:
  * Create, decode, refresh, and revoke API keys
  * Create, list, and delete custom roles
* Momento Serverless Cache (legacy):
  * Create, list, and delete caches
  * Set or get values from a cache
* Momento Topics:
  * Publish to and subscribe to topics
* Momento Functions:
  * Create, list, invoke, and delete functions
* [Inspect your cloud footprint for common inefficiencies](https://docs.momentohq.com/cloud-linter)

## Prerequisites

First things first - go to the [Momento Console](https://console.gomomento.com) to sign up. In the keys tab, generate your first API key/token to use with the CLI.

This token uniquely identifies your Momento interactions. The token should be treated like a sensitive password and all essential care must be taken to ensure its secrecy. We recommend that you store this token in a secret vault like AWS Secrets Manager. See the [docs](https://docs.momentohq.com/topics/authentication/api-keys) for more information on Momento API keys.

You'll also need your [endpoint URL](https://docs.momentohq.com/platform/regions).

## Installation

### Mac (intel or apple silicon)

```
brew tap momentohq/tap
brew install momento-cli
```

#### Upgrading to a newer version

```
brew upgrade momento-cli
```

### Linux

Visit the web page for the latest [github release](https://github.com/momentohq/momento-cli/releases).
There, you will find `.deb` and `.rpm` files for both x86_64 and aarch64.

`.deb` files have been tested on modern versions of Ubuntu and Debian.
`.rpm` files have been tested on modern versions of RHEL, Amazon Linux 2, Rocky Linux, and CentOS.

If you have problems with any of these packages on your favorite platform, please file an issue and let us know!

We also provide tarballs for both x86_64 and aarch64; these contain the `momento` binary,
which you may add anywhere you like in your execution path.

### Windows

Our CLI is available in the Windows Package Manager (`winget`). To install, run the following from PowerShell or the Command Prompt:

```powershell
winget install momento.cli
```

Alternatively visit the web page for the latest [github release](https://github.com/momentohq/momento-cli/releases).
There you will find an `.msi` installer for Windows platforms, as well as a windows `.zip` file if
you prefer to manually copy the `momento` executable to your preferred location.

If you have problems with the windows packages please file an issue and let us know!

### Tab Completion

We provide tab completion configuration for zsh and bash.  For linux installs via deb/rpm, no additional shell
configuration should be necessary if you already have zsh/bash tab completion enabled.  For Homebrew, see
https://docs.brew.sh/Shell-Completion

## Quick Start

These instructions assume you have the `momento` executable on your path, after following
the appropriate installation steps above.

```sh
# Configure your account with the API key and endpoint from above
momento configure

# Create your default cache
momento cache set-default default-cache --ttl 600

# Set and Get values from your default cache, with default ttl
momento cache set key value
momento cache get key

# Make a different cache
momento cache create example-cache

# Set and Get values from a non-default cache with a different ttl
momento cache set key value --ttl 100 --cache example-cache
momento cache get key --cache example-cache
```

## Configure

### First time configuration

```
# default profile name is default
momento configure
```

This will prompt you for your Momento API key (v2) and endpoint, and save them to be reused as a part of your `default` profile.

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
momento cache create example-cache
momento cache set key value --ttl 100 --cache example-cache
momento cache get key --cache example-cache
```

You can also specify your desired profile.

```
# use new-profile
momento cache create example-cache --profile new-profile
momento cache set key value --ttl 100 --cache example-cache --profile new-profile
momento cache get key --cache example-cache --profile new-profile
```

## Use Momento in Your Project

Check out our [SDKs](https://github.com/momentohq/client-sdk-examples) to integrate Momento into your project!

## Contributing

If you would like to contribute to the Momento Cli, please read out [Contributing Guide](./CONTRIBUTING.md)
