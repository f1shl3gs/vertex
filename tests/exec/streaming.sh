#!/bin/bash

for i in {0..5}; do
  echo "stdout $i"
  >&2 echo "stderr $i"
  sleep 1
done

if [ -n "$1" ]; then
  exit "$1"
fi
