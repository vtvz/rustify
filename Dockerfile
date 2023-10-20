FROM rustlang/rust:nightly as build

RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  cargo install sccache && mkdir -p /var/sccache

ENV SCCACHE_DIR /var/sccache
ENV RUSTC_WRAPPER sccache

# create a new empty shell project
RUN USER=root cargo new --bin rustify
WORKDIR /rustify

# copy over your manifests
COPY ./rust-toolchain.toml ./Cargo.lock ./Cargo.toml ./

RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/var/sccache \
  cargo build --release --target=x86_64-unknown-linux-gnu

RUN rm -rf /rustify*

COPY . .

RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/var/sccache \
  cargo build --release --target=x86_64-unknown-linux-gnu

FROM debian:11-slim

LABEL org.opencontainers.image.source=https://github.com/vtvz/rustify

ARG EXECUTABLE_PATH

RUN \
  --mount=type=cache,target=/var/cache/apt \
  apt-get update && apt-get install --no-install-recommends -y ca-certificates \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=build /rustify/target/x86_64-unknown-linux-gnu/release/rustify /usr/local/bin/

CMD ["/usr/local/bin/rustify"]
