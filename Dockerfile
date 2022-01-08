FROM debian:11-slim

RUN apt-get update && apt-get install -y ca-certificates

COPY ./target/release/rustify /usr/local/bin/

CMD /usr/local/bin/rustify
