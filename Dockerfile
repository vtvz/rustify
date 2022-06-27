FROM debian:11-slim

ARG EXECUTABLE_PATH

RUN apt-get update && apt-get install --no-install-recommends -y ca-certificates proxychains-ng \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY ./${EXECUTABLE_PATH} /usr/local/bin/

CMD ["/usr/local/bin/rustify"]
