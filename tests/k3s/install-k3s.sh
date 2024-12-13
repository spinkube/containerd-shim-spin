#!/bin/bash

set -euo pipefail

# Install k3s
# Since the latest k3s already bakes in the Spin shim, we don't need to reconfigure it.
# We will just need to make sure that containerd-shim-spin-v2 binary is in PATH
# and that the k3s service is running.

curl -sfL https://get.k3s.io | sh -
sudo systemctl start k3s
sudo k3s kubectl get nodes

sudo chmod 644 /etc/rancher/k3s/k3s.yaml
sudo cp /etc/rancher/k3s/k3s.yaml $HOME/.kube/k3s.yaml
export KUBECONFIG=$HOME/.kube/k3s.yaml
kubectl get nodes