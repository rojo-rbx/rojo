# Build Stage
# FROM ghcr.io/evanrichter/cargo-fuzz:latest AS BUILDER
# Note: evanrichter/cargo-fuzz:latest updates every week.
# Note: It was outdated at the time of writing this.
# Note: Switch back to the above when it's updated.

# SNIP
FROM rustlang/rust:nightly as BUILDER

RUN apt update && apt upgrade -y && \
    apt install -y clang-11 llvm-11-tools && \
    ln -s /usr/bin/llvm-config-11 /usr/bin/llvm-config

RUN rustup component add rust-src
RUN cargo install -f cargo-fuzz
RUN cargo install -f afl
# SNIP

# Add source code to the build stage.
ADD . /src
WORKDIR /src

# Create necessary directories (search 'Wally' for more info)
RUN mkdir /src/plugin/Packages
RUN mkdir /src/plugin/DevPackages

# Compile the fuzzers
RUN cargo +nightly fuzz build

# Package stage
FROM ubuntu:latest AS PACKAGE

# Copy the fuzzers to the final image
COPY --from=BUILDER /src/./fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_* /fuzzers/