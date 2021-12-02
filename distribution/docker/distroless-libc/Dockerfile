FROM gcr.azk8s.cn/distroless/cc-debian10

COPY vertex /usr/bin/vertex
COPY vertex.yml /etc/vertex/vertex.yml

VOLUME /var/lib/vertex

RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]