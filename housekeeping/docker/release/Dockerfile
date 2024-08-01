FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates nano curl && \
    apt-get autoremove -y && \
    apt-get clean && \
    useradd substrate -u 10000 && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete

USER substrate

COPY ./target/release/bridge-relayer /usr/local/bin/relayer

COPY ./target/release/bridge-relayer /usr/local/bin/relayer