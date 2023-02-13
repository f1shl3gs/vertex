FROM ghcr.io/cross-rs/x86_64-unknown-linux-musl:0.2.5

COPY bootstrap-ubuntu.sh .
RUN ./bootstrap-ubuntu.sh
