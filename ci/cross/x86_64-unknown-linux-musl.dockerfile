FROM docker.io/rustembedded/cross:x86_64-unknown-linux-musl

COPY bootstrap-ubuntu.sh .
RUN ./bootstrap-ubuntu.sh
COPY extra_musl_packages.sh .
RUN ./extra_musl_packages.sh