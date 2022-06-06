FROM ubuntu:22.10

COPY vertex /usr/bin/vertex

VOLUME /var/lib/vertex

RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]