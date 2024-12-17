#!/bin/bash

set -euo pipefail

# Install k3s
# Since the latest k3s already bakes in the Spin shim, we don't need to reconfigure it.
# We will just need to make sure that containerd-shim-spin-v2 binary is in PATH
# and that the k3s service is running.

curl -sfL https://get.k3s.io | sh -s - server --write-kubeconfig-mode '0644' 
sudo systemctl start k3s

export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
kubectl get nodes