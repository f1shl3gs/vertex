#!/bin/bash

date
date 1>&2

if [ -n "$1" ]; then
  exit "$1"
fi
