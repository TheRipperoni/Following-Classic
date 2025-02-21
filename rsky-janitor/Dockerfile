# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM --platform=linux/amd64 rust AS builder

# Copy local code to the container image.
WORKDIR /usr/src/rsky
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY rsky-lexicon rsky-lexicon
COPY rsky-identity rsky-identity
COPY rsky-syntax rsky-syntax
COPY rsky-feedgen rsky-feedgen
COPY rsky-janitor/Cargo.toml rsky-janitor/Cargo.toml
COPY rsky-crypto rsky-crypto
COPY rsky-firehose rsky-firehose

# Create an empty src directory to trick Cargo into thinking it's a valid Rust project
RUN mkdir rsky-janitor/src && echo "fn main() {}" > rsky-janitor/src/main.rs

## Install production dependencies and build a release artifact.
RUN cargo build --release --package rsky-janitor

COPY rsky-janitor/src rsky-janitor/src

RUN cargo build --package rsky-janitor

FROM --platform=linux/amd64 rust

WORKDIR /usr/src/rsky

COPY --from=builder /usr/src/rsky/target/debug/rsky-janitor rsky-janitor

CMD ["./rsky-janitor"]