# Momento CLI

Inglês: [English](README.md)
Japonês: [日本語](README.ja.md)

## Introdução
```
# Instalação
brew tap momentohq/tap
brew install momento-cli

## AWS [regiões disponíveis são us-west-2, us-east-1, ap-south-1, ap-northeast-1, eu-west-1]
momento account signup aws --email <insira_seu_email_aqui> --region <regiao_desejada>

## GCP [regiões disponíveis são us-east4, us-central1, ap-northeast1]
momento account signup gcp --email <insira_seu_email_aqui> --region <regiao_desejada>

# Configure sua conta com as credenciais recebidas por email
momento configure

# Crie um primeiro cache
momento cache create example-cache

# escreva e leia valores do seu cache
momento cache set key value --ttl 100 --cache example-cache
momento cache get key --cache example-cache

```

## Atualização de versão
```
brew update momento-cli
brew upgrade momento-cli
```

## Cadastro


**NOTA:** Se você encontrar erros durante o cadastro, por favor garanta que você atualizou para [latest version](https://github.com/momentohq/momento-cli/releases/latest) da nossa ferramenta de linha de comando (CLI).

### Momento na AWS

```
# Cheque a ajuda para ver todas as regiões disponíveis e se cadastre para uma região específica
momento account signup aws --help
momento account signup aws --email <insira_seu_email_aqui> --region <regiao_desejada>

# Configure a linha de comando (CLI)
momento configure

```

### Momento na GCP

```
# Cheque a ajuda para ver todas as regiões disponíveis e se cadastre para uma região específica
momento account signup gcp --help
momento account signup gcp --email <insira_seu_email_aqui> --region <regiao_desejada>

# Configure a linha de comando (CLI)
momento configure

```

Durante o cadastro, Momento envia um token para o email informado. Este token identifica unicamente as interações com o cache. O token deve ser tratado como dado sensível, assim como uma senha. Todo cuidado deve ser tomado para garantir que ele não vaze. Recomendamos que você armazene este token em um cofre como o AWS Secrets Manager.

## Configuração

### Configuração pela primeira vez

```
# nome do perfil padrão é default
momento configure
```

Este comando vai demandar o Momento Auth Token, nome do cache padrão, TTL padrão, e salvá-los para serem reusados como parte do perfil `default`.

```
momento configure --profile new-profile
```

Este comando vai te requisitar as mesmas informações anteriores e salvá-los para serem reusados como parte do perfil `new-profile`.

<br>

### Atualizar configuração existente

Para atualizar sem perfil, use os mesmos comandos anteriores.

## Use CLI

```
# use default profile
momento cache create example-cache
momento cache set key value --ttl 100 --cache example-cache
momento cache get key --cache example-cache
```

Você também pode especificar seu perfil desejado.

```
# usar new-profile
momento cache create example-cache --profile new-profile
momento cache set key value --ttl 100 --cache example-cache --profile new-profile
momento cache get key --cache example-cache --profile new-profile
```

## Usar Momento no seu Projeto

Confira nossos [SDKs](https://github.com/momentohq/client-sdk-examples) para integrar o Momento nos seus projetos!

## Contributing

Se você quiser contribuir com a CLI do Momento, por favor leia [Guia de Contribuições](./CONTRIBUTING.pt.md)
