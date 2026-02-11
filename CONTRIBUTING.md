### Building

```
cargo build
```

### Setup

In the [Momento Console](https://console.gomomento.com/), generate an API key. For the automated tests, a [legacy API key](https://console.gomomento.com/api-keys) is required with the following settings:
- **Type of key**: Super User Key
- **Expiration**: highly recommended (Legacy keys do not support revocation.)

Make sure you have `~/.momento/credentials` and `~/.momento/config` files with the following data.

`~/.momento/credentials`

```
[default]
token=<YOUR_TOKEN>
[YOUR_TEST_PROFILE]
token=<YOUR_TOKEN>
```

`~/.momento/config`

```
[default]
cache=<YOUR_TEST_CACHE_DEFAULT>
ttl=600
[YOUR_TEST_PROFILE]
cache=<YOUR_TEST_CACHE_WITH_PROFILE>
ttl=700
```

### Testing

```
read -p "Token: " TEST_AUTH_TOKEN
# Enter <YOUR_TOKEN> from above
export TEST_AUTH_TOKEN

./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

### Deploying

After merge a pull request (PR) will be created in this repo https://github.com/momentohq/homebrew-tap. Once the PR passes all checks, approve the PR and label it as `pr-pull`. It will then get automatically merged by the homebrew bot, and a release will be created for it.
