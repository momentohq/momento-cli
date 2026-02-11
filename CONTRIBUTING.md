### Building

```
make build
```

### Manual Testing

Make sure you have `~/.momento/credentials` and `~/.momento/config` files with your [API key(s)](https://console.gomomento.com/keys), [endpoint URL(s)](https://docs.momentohq.com/platform/regions), and [cache name(s)](https://console.gomomento.com/caches).

`~/.momento/credentials`

```
[default]
api_key_v2=<YOUR_TOKEN>
endpoint=<YOUR_ENDPOINT_URL>
[YOUR_TEST_PROFILE]
api_key_v2=<YOUR_TOKEN>
endpoint=<YOUR_ENDPOINT_URL>
```

- If you prefer, create a legacy API key instead (as for [automated testing](#automated-testing)), then set a `token` instead of `api_key_v2`/`endpoint`.

`~/.momento/config`

```
[default]
cache=<YOUR_TEST_CACHE_DEFAULT>
ttl=600
[YOUR_TEST_PROFILE]
cache=<YOUR_TEST_CACHE_WITH_PROFILE>
ttl=700
```

Follow the [README](./README.md#use-cli), using `./target/debug/momento` instead of `momento`, for example:

```bash
./target/debug/momento cache create example-cache
```

### Automated Testing

For the automated tests, a [legacy API key](https://console.gomomento.com/api-keys) is required with the following settings:
- **Type of key**: Super User Key
- **Expiration**: highly recommended (Legacy keys do not support revocation.)

```bash
read -s -p "API key: " TEST_AUTH_TOKEN
# Paste your API key. (Note: You will not be able to see it in the shell.)
export TEST_AUTH_TOKEN
make test
```

### Formatting

```bash
make lint
```

### Deploying

After merge a pull request (PR) will be created in this repo https://github.com/momentohq/homebrew-tap. Once the PR passes all checks, approve the PR and label it as `pr-pull`. It will then get automatically merged by the homebrew bot, and a release will be created for it.
