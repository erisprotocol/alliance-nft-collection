#!/usr/bin/env bash

# Running coverage
# docker run --security-opt seccomp=unconfined -v "/${PWD}:/volume" xd009642/tarpaulin sh -c "cargo tarpaulin --out lcov"

set -e
set -o pipefail

projectPath=$(dirname `pwd`) 
folderName=$(basename $(dirname `pwd`)) 

mkdir -p "../../$folderName-cache"
mkdir -p "../../$folderName-cache/target"
mkdir -p "../../$folderName-cache/registry"

docker run --env $1 --rm -v "/$projectPath":/code \
  --mount type=bind,source=/$projectPath-cache/target,target=/target \
  --mount type=bind,source=/$projectPath-cache/registry,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.15.1 