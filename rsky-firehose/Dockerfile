# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM --platform=linux/amd64 rust AS builder

# Copy local code to the container image.
WORKDIR /usr/src/rsky
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY rsky-lexicon rsky-lexicon
COPY rsky-identity rsky-identity
COPY rsky-syntax rsky-syntax
COPY rsky-janitor rsky-janitor
COPY rsky-firehose/Cargo.toml rsky-firehose/Cargo.toml
COPY rsky-crypto rsky-crypto
COPY rsky-feedgen rsky-feedgen

# Create an empty src directory to trick Cargo into thinking it's a valid Rust project
RUN mkdir rsky-firehose/src && echo "fn main() {}" > rsky-firehose/src/main.rs

## Install production dependencies and build a release artifact.
RUN cargo build --release --package rsky-firehose

COPY rsky-firehose/src rsky-firehose/src

RUN cargo build --release --package rsky-firehose


FROM --platform=linux/amd64 rust

WORKDIR /usr/src/rsky

COPY --from=builder /usr/src/rsky/target/release/rsky-firehose rsky-firehose

CMD ["./rsky-firehose"]