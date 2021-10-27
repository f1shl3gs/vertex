FROM alpine:3.13.6

ENV RUSTUP_DIST_SERVER="https://mirrors.ustc.edu.cn/rust-static"
ENV RUSTUP_UPDATE_ROOT="https://mirrors.ustc.edu.cn/rust-static/rustup"
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories
RUN apk add --no-cache curl libgcc
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o install.sh && sh ./install.sh -y

