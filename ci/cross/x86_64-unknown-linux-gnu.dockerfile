FROM ghcr.io/cross-rs/x86_64-unknown-linux-gnu:main-centos

COPY bootstrap-ubuntu.sh .
RUN ./bootstrap-ubuntu.sh