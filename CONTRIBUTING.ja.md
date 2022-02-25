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

```
cargo test
```

## デプロイ

マージ後こちらのリポジトリにプルリクエストが作成されます。https://github.com/momentohq/homebrew-tap
プルリクエストに対し全てのチェックが完了、合格した後、`pr-pull`というラベルで承認いたします。その後、homebrew ボットにより自動的にマージされ、リリースが作成されます。
