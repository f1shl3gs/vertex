ARG UBUNTU_RELEASE=24.10

FROM ubuntu:$UBUNTU_RELEASE AS builder

ARG CHISEL_VERSION=v1.2.0
ARG TARGETARCH UBUNTU_RELEASE

SHELL ["/bin/bash", "-oeux", "pipefail", "-c"]

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

ADD "https://github.com/canonical/chisel/releases/download/${CHISEL_VERSION}/chisel_${CHISEL_VERSION}_linux_${TARGETARCH}.tar.gz" \
    chisel.tar.gz

RUN tar -xvf chisel.tar.gz -C /usr/bin/

RUN mkdir /rootfs \
    && chisel cut --release "ubuntu-$UBUNTU_RELEASE" --root /rootfs \
        libgcc-s1_libs \
        libc6_libs \
        ca-certificates_data

FROM scratch

COPY --from=builder /rootfs /

COPY vertex /usr/bin/vertex

VOLUME /var/lib/vertex

# ensure vertex can run in this docker image
RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]