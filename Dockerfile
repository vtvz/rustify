FROM rustlang/rust:nightly-bookworm AS build

# create a new empty shell project
RUN USER=root cargo new --bin rustify
WORKDIR /rustify

# copy over your manifests
COPY ./rust-toolchain.toml ./

# for installing toolchain
RUN rustup show

COPY ./Cargo.lock ./Cargo.toml ./build.rs ./

RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/rustify/target \
  cargo build --release --target=x86_64-unknown-linux-gnu

RUN rm -rf /rustify*

COPY . .

ARG GIT_COMMIT_TIMESTAMP
ENV GIT_COMMIT_TIMESTAMP=${GIT_COMMIT_TIMESTAMP}

ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}

RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/rustify/target \
  cargo build --release --target=x86_64-unknown-linux-gnu && mv target/x86_64-unknown-linux-gnu/release/rustify rustify

FROM debian:bookworm

LABEL org.opencontainers.image.source=https://github.com/vtvz/rustify

ARG EXECUTABLE_PATH

RUN \
  --mount=type=cache,target=/var/cache/apt \
  apt-get update && apt-get install --no-install-recommends -y ca-certificates \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=build /rustify/rustify /usr/local/bin/

CMD ["/usr/local/bin/rustify"]
