FROM rust:1.83.0 AS builder

# Set the working directory inside the container
WORKDIR /usr/src/rustify

# copy over your manifests
COPY ./rust-toolchain.toml ./

# for installing toolchain
RUN rustup show

# Cache dependencies. First, copy the Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to ensure `cargo build` can succeed for dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Fetch dependencies without building the actual project (this will be cached)

RUN cargo fetch
RUN cargo build --release

# Copy the rest of the source code and build
COPY . .

ARG GIT_COMMIT_TIMESTAMP
ENV GIT_COMMIT_TIMESTAMP=${GIT_COMMIT_TIMESTAMP}

ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}

RUN touch src/main.rs && cargo build --release

# Use a minimal base image for the runtime
FROM debian:bookworm-slim

RUN \
  --mount=type=cache,target=/var/cache/apt \
  apt-get update && apt-get install --no-install-recommends -y ca-certificates \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the build stage
COPY --from=builder /usr/src/rustify/target/release/rustify /usr/local/bin/rustify

ENTRYPOINT [ "/usr/local/bin/rustify" ]
