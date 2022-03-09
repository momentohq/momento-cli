_他言語バージョンもあります_: [English](README.md)

<br>

## 必要条件

- MacOS もしくは Linux
- [Homebrew](https://brew.sh/)

## インストール方法

```
brew tap momentohq/tap
brew install momento-cli
```

## サインアップ方法

**注意:** サインアップ中にエラーが発生した場合は、CLI のバージョンを[最新バージョン](https://github.com/momentohq/momento-cli/releases/latest)に更新して下さい。

```
# デフォルトのリージョンはus-west-2です
momento account signup --email <ご自身のメールアドレス>

# (オプション) help機能を使って、利用可能なリージョンを確認し、サインアップの際に特定のリージョンを選択して下さい。
momento account signup --help
momento account signup --email <ご自身のメールアドレス> --region <ご希望のリージョン>
```

上記のコマンドはアクセストークンを発行し、提供していただいたメールアドレスに送付します。こちらのトークンは独自にキャッシュインタラクションを識別します。トークンはセンシティブなパスワードの様に扱ってください。また、秘密を確信するため全ての必要不可欠な対応をお願いします。AWS Secrets Manager の様なシークレット管理サービスにトークンを保管する事をお勧めします。

## コンフィギュア

```

momento configure

```

上記コマンドは Momento オーストークン、デフォルトのキャッシュ名、デフォルトの TTL の入力を要求します。入力後、トークンは保存され、あなたの”デフォルト”プロファイルとして使用されます。

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
