FROM rustlang/rust:nightly-bookworm AS builder

# Set the working directory inside the container
WORKDIR /usr/src/rustify

# copy over your manifests
COPY ./rust-toolchain.toml ./

# for installing toolchain
RUN rustup show

# Cache dependencies. First, copy the Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to ensure `cargo build` can succeed for dependencies
RUN mkdir -p src/bin \
  && echo "fn main() {}" > src/bin/bot.rs \
  && echo "fn main() {}" > src/bin/metrics.rs \
  && echo "fn main() {}" > src/bin/track_check.rs

# Fetch dependencies without building the actual project (this will be cached)

RUN cargo fetch
RUN cargo build --release

# Copy the rest of the source code and build
COPY . .

ARG GIT_COMMIT_TIMESTAMP
ENV GIT_COMMIT_TIMESTAMP=${GIT_COMMIT_TIMESTAMP}

ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}

RUN cargo build --release

# Use a minimal base image for the runtime
FROM debian:bookworm-slim

RUN \
  --mount=type=cache,target=/var/cache/apt \
  apt-get update && apt-get install --no-install-recommends -y ca-certificates \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the build stage
COPY --from=builder /usr/src/rustify/target/release/bot /usr/local/bin/rustify-bot
COPY --from=builder /usr/src/rustify/target/release/metrics /usr/local/bin/rustify-metrics
COPY --from=builder /usr/src/rustify/target/release/track_check /usr/local/bin/rustify-track-check
