## Construindo

```
make build
```

## Manual Testing

Garanta que você tem os arquivos `~/.momento/credentials` e `~/.momento/config` com os seguintes dados.

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

## Automated Testing

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

### Implantação

Depois do merge, um pull request será criado no repositório https://github.com/momentohq/homebrew-tap. Depois que o pull request passar por todas as validações, aprove o PR e aplique nele a tag `pr-pull`. Ele será então automaticamente mergeado pelo bot do homebrew e uma versão será criada para ele.