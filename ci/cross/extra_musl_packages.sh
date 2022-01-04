#!/usr/bin/env bash

set -euo pipefail

# Start building
#
# Master is not a good choice, but it can be build properly
wget https://gitlab.com/libvirt/libvirt/-/archive/master/libvirt-master.tar.gz
wget https://gitlab.com/qemu-project/keycodemapdb/-/archive/27acf0ef828bf719b2053ba398b195829413dbdd/keycodemapdb-27acf0ef828bf719b2053ba398b195829413dbdd.tar.gz -O keycodemapdb.tgz
tar -xf libvirt-master.tar.gz
tar -zxf keycodemapdb.tgz && cp -r keycodemapdb-27acf0ef828bf719b2053ba398b195829413dbdd/* libvirt-master/src/keycodemapdb
cd libvirt-master
meson configure --prefix /usr/local/x86_64-linux-musl
CC=/usr/local/bin/x86_64-linux-musl-gcc meson build -Ddocs=disabled -Ddriver_remote=disabled && CC=/usr/local/bin/x86_64-linux-musl-gcc ninja -C build install
rm -rf libvirt* keycodemapdb*


#  meson configure --prefix /usr/local/x86_64-linux-musl && \
#  CC=/usr/local/bin/x86_64-linux-musl-gcc meson build