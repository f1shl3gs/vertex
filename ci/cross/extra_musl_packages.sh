#!/usr/bin/env bash

set -e -o verbose

# Start building
wget https://gitlab.com/libvirt/libvirt/-/archive/v7.10.0/libvirt-v7.10.0.tar.gz -O libvirt.tgz
wget https://gitlab.com/qemu-project/keycodemapdb/-/archive/27acf0ef828bf719b2053ba398b195829413dbdd/keycodemapdb-27acf0ef828bf719b2053ba398b195829413dbdd.tar.gz -O keycodemapdb.tgz
ls
tar -zxf libvirt.tgz
ls
tar -zxf keycodemapdb.tgz && cp -r keycodemapdb-27acf0ef828bf719b2053ba398b195829413dbdd/* libvirt-v7.10.0/src/keycodemapdb
cd libvirt-v7.10.0
meson build -Ddocs=disabled -Ddriver_remote=disabled -Dsystem=false && ninja -C build install
rm -rf libvirt* keycodemapdb*


#  meson configure --prefix /usr/local/x86_64-linux-musl && \
#  CC=/usr/local/bin/x86_64-linux-musl-gcc meson build