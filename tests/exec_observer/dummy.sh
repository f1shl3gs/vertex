#!/usr/bin/env bash

cat << EOF
[
  {
    "id": "abcd",
    "target": "127.0.0.1:5333",
    "type": "service",
    "details": {
      "foo": "bar"
    }
  }
]
EOF
