FROM docker.io/alpine:3.16.0

COPY vertex /usr/bin/vertex
VOLUME /var/lib/vertex

RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]