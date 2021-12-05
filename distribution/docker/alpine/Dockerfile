FROM docker.io/alpine:3.15.0

COPY vertex /usr/bin/vertex
COPY vertex.yml /etc/vertex/vertex.yml

VOLUME /var/lib/vertex

RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]