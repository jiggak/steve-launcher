FROM rust:latest AS builder

ARG MSA_CLIENT_ID
ARG CURSE_API_KEY

RUN test -n "${MSA_CLIENT_ID}"
RUN test -n "${CURSE_API_KEY}"

WORKDIR /app

# Copy Cargo files first to leverage Docker cache for dependencies
COPY Cargo.toml Cargo.lock ./
COPY steve-cli/Cargo.toml steve-cli/

# Create dummy source files to build dependencies separate from app code
RUN mkdir src && echo "pub fn dummy() {}" > src/lib.rs \
 && mkdir steve-cli/src && echo "fn main() {}" > steve-cli/src/main.rs

# Build dependencies as separate layer to optimize build time when only code changes
RUN cargo build --release -p steve-cli

COPY src src
COPY steve-cli steve-cli

# Hack to prevent docker from using the dummy code
RUN touch -a -m src/lib.rs src/main.rs

RUN MSA_CLIENT_ID=${MSA_CLIENT_ID} CURSE_API_KEY=${CURSE_API_KEY} \
    cargo build --release -p steve-cli

FROM eclipse-temurin:21-jre

ENV STEVE_DATA_HOME=/steve/.data

COPY --from=builder /app/target/release/steve /app/steve

VOLUME ["/steve"]
WORKDIR /steve

EXPOSE 25565

ENTRYPOINT ["/app/steve"]
CMD ["launch"]
