#!/bin/bash

VERSION="DSP2043_2024.3"

if [ ! -d "tests/redfish/${VERSION}" ]; then
  wget https://www.dmtf.org/sites/default/files/standards/documents/${VERSION}.zip -O tests/redfish/${VERSION}.zip && unzip -qq tests/redfish/${VERSION}.zip -d tests/redfish
fi
