FROM debian:bullseye-20240722-slim

ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates nano curl libc6 libstdc++6 && \
    apt-get autoremove -y && \
    apt-get clean && \
    useradd substrate -u 10000 && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete

USER substrate

COPY ./target/release/bridge-relayer /usr/local/bin/relayer