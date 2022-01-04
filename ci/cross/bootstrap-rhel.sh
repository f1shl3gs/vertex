#!/bin/sh

yum makecache

# we need LLVM >= 3.9 for onig_sys/bindgen
# and perl-IPC-Cmd for OpenSSL

yum install -y centos-release-scl \
  llvm-toolset-7 \
  perl-IPC-Cmd \
  libvirt-devel

yum clean all