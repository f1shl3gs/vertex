FROM ubuntu:jammy

RUN apt-get install -yq \
        build-essential \
        curl \
        git \
        musl-dev \
        musl-tools \
        linux-libc-dev \
        pkgconf \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \

# Static linking for C++ code
RUN ln -s "/usr/bin/g++" "/usr/bin/musl-g++"

# Set up our path with all our binary directories, including those for the musl-gcc
# toolchain and for our Rust toolchain.
#
# We use the instructions at https://github.com/rust-lang/rustup/issues/2383
# to install the rustup toolchain as root
ENV RUSTUP_HOME=/opt/rust/rustup \
    PATH=/home/rust/.cargo/bin:/opt/rust/cargo/bin:/usr/local/musl/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

