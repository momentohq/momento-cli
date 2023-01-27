#!/bin/bash
set -x

VERSION=21.9

function install_protoc() {
    os=$1
    mkdir temp
    pushd temp
        curl -L https://github.com/protocolbuffers/protobuf/releases/download/v$VERSION/protoc-$VERSION-$os.zip -o protoc.zip
        unzip -o protoc.zip -d protoc
        sudo mv protoc/bin/* /usr/local/bin/
        sudo mv protoc/include/* /usr/local/include/
    popd
    rm -rf temp
}
