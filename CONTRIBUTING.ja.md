## ビルド

```
cargo build
```

## Setup

In the [Momento Console](https://console.gomomento.com/), generate an API key. For the automated tests, a [legacy API key](https://console.gomomento.com/api-keys) is required with the following settings:
- **Type of key**: Super User Key
- **Expiration**: highly recommended (Legacy keys do not support revocation.)

`~/.momento/credentials` と `~/.momento/config` が以下のデータを含み、存在している事をご確認ください。

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

## テスト

```
read -p "Token: " TEST_AUTH_TOKEN
# Enter <YOUR_TOKEN> from above
export TEST_AUTH_TOKEN

./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

## デプロイ

マージ後こちらのリポジトリにプルリクエストが作成されます。https://github.com/momentohq/homebrew-tap
プルリクエストに対し全てのチェックが完了、合格した後、`pr-pull`というラベルで承認いたします。その後、homebrew ボットにより自動的にマージされ、リリースが作成されます。
