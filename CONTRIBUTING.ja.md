## ビルド

```
cargo build
```

## テスト

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

```
export TEST_CACHE_DEFAULT=<YOUR_TEST_CACHE_DEFAULT>
export TEST_CACHE_WITH_PROFILE=<YOUR_TEST_CACHE_WITH_PROFILE>
export TEST_PROFILE=<YOUR_TEST_PROFILE>
./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

<br>

:warning: `cargo test --test configure_profiles_test`を実行する上での注意事項

```
export TEST_AUTH_TOKEN=<YOUR_TEST_AUTH_TOKEN>
cargo test --test configure_profile_test
```

- もし credentials と config ファイルがすでにローカル環境に存在する場合、`cargo test --test configure_profiles_test`を実行する事により`TEST_AUTH_TOKEN_DEFAULT`で指定されたトークンがご自身の`default`プロファイルで指定されたトークン値を上書きします。
- `TEST_CACHE_DEFAULT`の値とご自身の`default`プロファルの`cache`値が同じである事、またその`cache`がすでに存在する事が必須条件です。しかし、このテストの実行が成功した場合、その`cache`は削除されます。

## デプロイ

マージ後こちらのリポジトリにプルリクエストが作成されます。https://github.com/momentohq/homebrew-tap
プルリクエストに対し全てのチェックが完了、合格した後、`pr-pull`というラベルで承認いたします。その後、homebrew ボットにより自動的にマージされ、リリースが作成されます。
