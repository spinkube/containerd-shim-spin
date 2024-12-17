#!/bin/bash

set -euo pipefail

cluster_name="test-cluster"
dockerfile_path="deployments/k3d"

docker build -t k3d-shim-test "$dockerfile_path"

k3d cluster create "$cluster_name" \
  --image k3d-shim-test --api-port 6551 -p '8082:80@loadbalancer' --agents 2 \
  --registry-create test-registry:0.0.0.0:5000 \
  --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%@agent:*' \
  --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%@agent:*'

kubectl wait --for=condition=ready node --all --timeout=120s

echo "Running Spin and Docker builds and pushes..."
./scripts/spin-build-and-push-images.sh --both

echo ">>> Cluster setup and image builds/pushes complete!"