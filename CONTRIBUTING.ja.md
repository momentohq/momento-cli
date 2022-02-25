### 開始方法

1. git submodule init
1. git submodule sync
1. git submodule update --recursive --remote

## ビルド

1. cargo build

## テスト

1. cargo test

## デプロイ

マージ後こちらのリポジトリにプルリクエストが作成されます。https://github.com/momentohq/homebrew-tap
プルリクエストに対し全てのチェックが完了、合格した後、`pr-pull`というラベルで承認いたします。その後、homebrew ボットにより自動的にマージされ、リリースが作成されます。
