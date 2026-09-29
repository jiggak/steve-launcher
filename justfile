install:
    cargo install --path steve-cli --root ~/.local

docker-build:
    docker build -t steve . \
        --build-arg MSA_CLIENT_ID="${MSA_CLIENT_ID}" \
        --build-arg CURSE_API_KEY="${CURSE_API_KEY}"
