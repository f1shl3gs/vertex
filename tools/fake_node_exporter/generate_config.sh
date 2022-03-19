#!/usr/bin/env bash

set -euo pipefail

COUNT=$1
OUTPUT="prom_exporter_to_blackhole.yml"

echo "# This config is generated, do not edit.

sinks:
  blackhole:
    type: blackhole
    inputs:
      - prom
  exporter:
    type: prometheus_exporter
    inputs:
      - selfstat

sources:
  selfstat:
    type: selfstat
  prom:
    type: prometheus_scrape
    endpoints:" > $OUTPUT

for((i = 0; i < COUNT; i++)); do
  # The params is not necessary for fake_node_exporter, but vertex need it to shuffler
  # the scrap tasks
  echo "      - http://127.0.0.1:3000/metrics?index=${i}" >> $OUTPUT
done
