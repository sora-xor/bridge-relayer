FROM debian:bullseye
# Install dependencies
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates nano curl && \
    apt-get autoremove -y && \
    apt-get clean && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete

COPY ./target/release/bridge-relayer /usr/local/bin/relayer

ENTRYPOINT ["relayer"]
