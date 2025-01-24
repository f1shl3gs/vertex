FROM debian:trixie-slim

ADD target/release/vertex /

RUN mkdir -p /var/lib/vertex

CMD ["/vertex", "-c", "/etc/vertex/vertex.yaml"]
