FROM debian:trixie-slim

RUN mkdir -p /var/lib/vertex && sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources && apt update && apt install -y ca-certificates

ADD target/release/vertex /

CMD ["/vertex", "-c", "/etc/vertex/vertex.yaml"]
