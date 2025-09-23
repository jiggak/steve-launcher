#!/bin/bash

if [ -f .cargo/config.toml ]; then
   export $(cat .cargo/config.toml | grep 'MSA_CLIENT_ID\|CURSE_API_KEY' | xargs)
fi

docker build -t steve . \
   --build-arg MSA_CLIENT_ID="${MSA_CLIENT_ID}" \
   --build-arg CURSE_API_KEY="${CURSE_API_KEY}"
