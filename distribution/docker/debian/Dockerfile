FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

COPY vertex /usr/bin/vertex
COPY vertex.yml /etc/vertex/vertex.yml

VOLUME /var/lib/vertex

RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]