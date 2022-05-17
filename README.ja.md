_他言語バージョンもあります_: [English](README.md)、[Português](README.pt.md)



<br>

## クイックスタート

Linux のインストールマニュアルは[こちら](https://github.com/momentohq/momento-cli/blob/main/README.ja.md#linux)、Windows のインストールマニュアルは[こちら](https://github.com/momentohq/momento-cli/blob/main/README.ja.md#windows)を参照してください。

```
# インストール
brew tap momentohq/tap
brew install momento-cli

# サインアップ [available regions are us-west-2, us-east-1, ap-northeast-1, default is us-west-2]
momento account signup aws --email <TYPE_YOUR_EMAIL_HERE> --region <TYPE_DESIRED_REGION>

# 上記のメールアドレスに送付されたトークンとデフォルトのキャッシュ名とTTLであなたのアカウントコンフィグ
momento configure --quick

# キャッシュ作成
momento cache create --name example-cache

# キャッシュからSet・Getでバリューを取得
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache

```

## インストール

### Linux

1. 最新の linux tar.gz ファイルを[https://github.com/momentohq/momento-cli/releases/latest](https://github.com/momentohq/momento-cli/releases/latest)からダウンロードする。
2. `tar -xvf momento-cli-X.X.X.linux_x86_64.tar.gz`ファイルを展開する。
3. `./momento`を実行パスに置く。

### Windows

1. 最新の windows zip ファイルを[https://github.com/momentohq/momento-cli/releases/latest](https://github.com/momentohq/momento-cli/releases/latest)からダウンロードする。
2. `momento-cli-X.X.X.windows_x86_64.zip`ファイルを展開する。
3. 展開した.exe file を実行する。

## アップグレード

```
brew update momento-cli
brew upgrade momento-cli
```

## サインアップ方法

**注意:** サインアップ中にエラーが発生した場合は、CLI のバージョンを[最新バージョン](https://github.com/momentohq/momento-cli/releases/latest)に更新して下さい。

```
# デフォルトのリージョンはus-west-2です
momento account signup aws --email <ご自身のメールアドレス>

# (オプション) help機能を使って、利用可能なリージョンを確認し、サインアップの際に特定のリージョンを選択して下さい。
momento account signup --help
momento account signup --email <ご自身のメールアドレス> --region <ご希望のリージョン>
```

上記のコマンドはアクセストークンを発行し、提供していただいたメールアドレスに送付します。こちらのトークンは独自にキャッシュインタラクションを識別します。トークンはセンシティブなパスワードの様に扱ってください。また、秘密を確信するため全ての必要不可欠な対応をお願いします。AWS Secrets Manager の様なシークレット管理サービスにトークンを保管する事をお勧めします。

## コンフィグ

### 初回コンフィグ

```

momento configure

```

上記コマンドは Momento オーストークン、デフォルトのキャッシュ名、デフォルトの TTL の入力を要求します。入力後、トークンは保存され、あなたの”デフォルト”プロファイルとして使用されます。

```
momento configure --profile new-profile
```

上記コマンドも Momento オーストークン、デフォルトのキャッシュ名、デフォルトの TTL の入力を要求します。入力後、トークンは保存され、あなたの”new-profile”プロファイルとして使用されます。

<br>

### 既存のコンフィグをアップデート

ご希望のプロファイルをアップデートするには、上記と同様のコマンドをご使用ください。

## CLI 使用方法

```
#　デフォルトプロファイルが使用される
momento cache create --name example-cache
momento cache set --key key --value value --ttl 100 --name example-cache
momento cache get --key key --name example-cache

```

ご希望のプロファイルを指定する事もできます。

```
# new-profileが使用される
momento cache create --name example-cache --profile new-profile
momento cache set --key key --value value --ttl 100 --name example-cache --profile new-profile
momento cache get --key key --name example-cache --profile new-profile
```

## ご自身のプロジェクト内での Momento 使用方法

ご自身のプロジェクトに Momento をインテグレートする際には、ぜひ私達の[SDK](https://github.com/momentohq/client-sdk-examples)を確認してください！

## Momento CLI リポジトリへの貢献について

もし Momento CLI リポジトリへの貢献に興味がありましたら、こちらの[貢献ガイド](./CONTRIBUTING.ja.md)のご参考をお願いします。
