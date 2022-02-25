## 必要条件

- MacOS もしくは Linux
- [Homebrew](https://brew.sh/)

## インストール方法

```
brew tap momentohq/tap
brew install momento-cli
```

## サインアップ方法

現在の利用可能 AWS リージョン： `us-east-1` もしくは `us-west-2`

```
momento account signup --region <ご希望のリージョン> --email <メールアドレス>
```

上記のコマンドはアクセストークンを発行し、提供していただいたメールアドレスに送付します。こちらのトークンは独自にキャッシュインタラクションを識別します。トークンはセンシティブなパスワードの様に扱ってください。また、秘密を確信するため全ての必要不可欠な対応をお願いします。AWS Secrets Manager の様なシークレット管理サービスにトークンを保管する事をお勧めします。

## コンフィギュア

```
momento configure
```

上記コマンドは Momento オーストークンの入力を要求します。入力後はトークンは保存され、再利用されます。

## CLI 使用方法

```
momento cache create --name example-cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache
```

## ご自身のプロジェクト内での Momento 使用方法

ご自身のプロジェクトに Momento をインテグレートする際には、ぜひ私達の[SDK](https://github.com/momentohq/client-sdk-examples)を確認してください！

## Momento CLI リポジトリへの貢献について

もし Momento CLI リポジトリへの貢献に興味がありましたら、こちらの[貢献ガイド](./CONTRIBUTING.ja.md)のご参考をお願いします。
