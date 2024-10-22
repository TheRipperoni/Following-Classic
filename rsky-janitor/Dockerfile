# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM --platform=linux/amd64 rust

# Copy local code to the container image.
WORKDIR /usr/src/rsky
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY rsky-lexicon rsky-lexicon
COPY rsky-identity rsky-identity
COPY rsky-syntax rsky-syntax
COPY rsky-firehose rsky-firehose
COPY rsky-janitor/Cargo.toml rsky-janitor/Cargo.toml
COPY rsky-crypto rsky-crypto
COPY rsky-feedgen rsky-feedgen

COPY rsky-janitor/src rsky-janitor/src

RUN cargo build --package rsky-janitor

CMD cargo run --package rsky-janitor

