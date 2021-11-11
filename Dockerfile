# rust-musl-builder cannot build with Edition 2021, So for now
# centos is used, maybe we should build our own musl-image
FROM centos:centos8.4.2105
# FROM alpine:3.13.6

COPY ./target/release/vertex /bin/vertex

ENTRYPOINT ["/bin/vertex"]