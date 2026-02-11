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

export TEST_CACHE_DEFAULT=<YOUR_TEST_CACHE_DEFAULT>
export TEST_CACHE_WITH_PROFILE=<YOUR_TEST_CACHE_WITH_PROFILE>
export TEST_PROFILE=<YOUR_TEST_PROFILE>

./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

<br>

:warning: Important notes on running `cargo test --test configure_profiles_test`

```
export TEST_AUTH_TOKEN=<YOUR_TEST_AUTH_TOKEN>
cargo test --test configure_profile_test
```

- If you already have existing credentials and config files locally, running `cargo test --test configure_profiles_test` with provided `TEST_AUTH_TOKEN_DEFAULT` will overwrite the value for token in your `default` profile.
- The value for `TEST_CACHE_DEFAULT` needs to match the cache value in your `default` profile and the cache needs to exist. However, this cache will be deleted after this test runs successfully.

### Deploying

After merge a pull request (PR) will be created in this repo https://github.com/momentohq/homebrew-tap. Once the PR passes all checks, approve the PR and label it as `pr-pull`. It will then get automatically merged by the homebrew bot, and a release will be created for it.
