FROM debian:bullseye-20240513-slim

ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates nano curl && \
    apt-get autoremove -y && \
    apt-get clean && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete &&\
    useradd substrate -u 10000

USER substrate

COPY ./target/release/bridge-relayer /usr/local/bin/relayer