## Construindo

```
cargo build
```

## Setup

In the [Momento Console](https://console.gomomento.com/), generate an API key. For the automated tests, a [legacy API key](https://console.gomomento.com/api-keys) is required with the following settings:
- **Type of key**: Super User Key
- **Expiration**: highly recommended (Legacy keys do not support revocation.)

Garanta que você tem os arquivos `~/.momento/credentials` e `~/.momento/config` com os seguintes dados.

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

## Testando

```
read -p "Token: " TEST_AUTH_TOKEN
# Enter <YOUR_TOKEN> from above
export TEST_AUTH_TOKEN

./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

### Implantação

Depois do merge, um pull request será criado no repositório https://github.com/momentohq/homebrew-tap. Depois que o pull request passar por todas as validações, aprove o PR e aplique nele a tag `pr-pull`. Ele será então automaticamente mergeado pelo bot do homebrew e uma versão será criada para ele.