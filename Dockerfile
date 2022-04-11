FROM debian:11-slim

RUN apt-get update && apt-get install --no-install-recommends -y ca-certificates proxychains-ng \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY ./target/release/rustify /usr/local/bin/

CMD ["/usr/local/bin/rustify"]
