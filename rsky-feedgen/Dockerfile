## Builder
#FROM --platform=linux/amd64 rust
#
## Copy local code to the container image.
#WORKDIR /usr/src/rsky
#COPY Cargo.toml Cargo.lock rust-toolchain ./
#
## Install production dependencies and build a release artifact.
#RUN cargo build --release --package rsky-feedgen
#
## Run the web service on container startup.
#CMD ROCKET_PORT=$PORT ROCKET_ADDRESS=0.0.0.0 ROCKET_ENV=prod cargo run --release --package rsky-feedgen

## SPlit

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
COPY rsky-jetstream rsky-jetstream
COPY rsky-feedgen/Cargo.toml rsky-feedgen/Cargo.toml
COPY rsky-crypto rsky-crypto
COPY rsky-firehose rsky-firehose

# Create an empty src directory to trick Cargo into thinking it's a valid Rust project
RUN mkdir rsky-feedgen/src && echo "fn main() {}" > rsky-feedgen/src/main.rs

## Install production dependencies and build a release artifact.
RUN cargo build --release --package rsky-feedgen

COPY rsky-feedgen/src rsky-feedgen/src

RUN cargo build --release --package rsky-feedgen

FROM --platform=linux/amd64 rust

WORKDIR /usr/src/rsky

COPY --from=builder /usr/src/rsky/target/release/rsky-feedgen rsky-feedgen

CMD ["./rsky-feedgen"]