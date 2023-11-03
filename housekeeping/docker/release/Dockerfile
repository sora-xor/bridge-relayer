FROM debian:bullseye
# Install dependencies
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates nano curl && \
    apt-get autoremove -y && \
    apt-get clean && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete

COPY target /target

RUN useradd substrate -u 10000
RUN mkdir /chain && \
    chown 10000:10000 /chain
USER substrate

COPY ./target/release/bridge-relayer ./bridge-relayer

ENTRYPOINT ["bridge-relayer"]