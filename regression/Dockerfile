FROM debian:trixie-slim

RUN mkdir -p /var/lib/vertex && apt update && apt install -y ca-certificates

ADD target/release/vertex /

CMD ["/vertex", "-c", "/etc/vertex/vertex.yaml"]
