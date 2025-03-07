#!/bin/bash
sudo apt -y update
sudo apt-get install -y protobuf-compiler libseccomp-dev

## setup tinygo. required for building test spin app
echo "setting up tinygo"
wget https://github.com/tinygo-org/tinygo/releases/download/v0.34.0/tinygo_0.34.0_amd64.deb
sudo dpkg -i tinygo_0.34.0_amd64.deb
rm tinygo_0.34.0_amd64.deb
