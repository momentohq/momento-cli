## Construindo

```
cargo build
```

## Testando

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

```
export TEST_CACHE_DEFAULT=<YOUR_TEST_CACHE_DEFAULT>
export TEST_CACHE_WITH_PROFILE=<YOUR_TEST_CACHE_WITH_PROFILE>
export TEST_PROFILE=<YOUR_TEST_PROFILE>
./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

<br>

:warning: Observações importantes sobre a execução `cargo test --test configure_profiles_test`

```
export TEST_AUTH_TOKEN=<YOUR_TEST_AUTH_TOKEN>
cargo test --test configure_profile_test
```

- Se você já tem credenciais e arquivos de configuração existentes em ambiente local, executar `cargo test --test configure_profiles_test` com o token `TEST_AUTH_TOKEN_DEFAULT` disponibilizado vai sobrescrever o valor do token no seu perfil `default`.
- O valor do `TEST_CACHE_DEFAULT` precisa coincidir com o valor do cache no seu perfil `default` e o cache deve existir. Entretanto, este cache será removido após o teste ser executado com sucesso.
  
### Implantação

Depois do merge, um pull request será criado no repositório https://github.com/momentohq/homebrew-tap. Depois que o pull request passar por todas as validações, aprove o PR e aplique nele a tag `pr-pull`. Ele será então automaticamente mergeado pelo bot do homebrew e uma versão será criada para ele.