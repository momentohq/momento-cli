### 開始方法

```
git submodule init
git submodule sync
git submodule update --recursive --remote
```

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

## デプロイ

マージ後こちらのリポジトリにプルリクエストが作成されます。https://github.com/momentohq/homebrew-tap
プルリクエストに対し全てのチェックが完了、合格した後、`pr-pull`というラベルで承認いたします。その後、homebrew ボットにより自動的にマージされ、リリースが作成されます。
