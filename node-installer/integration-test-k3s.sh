#!/bin/bash
set -euo pipefail

echo "Installing K3s..."
curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="--disable=traefik --write-kubeconfig-mode=644" sh -

echo "Waiting for K3s to be ready..."
sleep 10
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
until kubectl get nodes | grep -q " Ready"; do
  echo "Waiting for node to be ready..."
  sleep 5
done

echo "=== Step 2: Create namespace and deploy RuntimeClass ==="
kubectl create namespace kwasm || true
kubectl apply -f ../deployments/workloads/runtime.yaml

echo "=== Step 3: Build and deploy the KWasm node installer ==="
if ! docker image inspect ghcr.io/spinkube/containerd-shim-spin/node-installer:dev >/dev/null 2>&1; then
  echo "Building node installer image..."
  PLATFORM=$(uname -m)
  if [ "$PLATFORM" = "x86_64" ]; then
    PLATFORM="linux/amd64"
    ARCH="x86_64"
  elif [ "$PLATFORM" = "aarch64" ] || [ "$PLATFORM" = "arm64" ]; then
    PLATFORM="linux/arm64"
    ARCH="aarch64"
  else
    echo "Unsupported platform: $PLATFORM"
    exit 1
  fi
  
  PLATFORM=$PLATFORM ARCH=$ARCH IMAGE_NAME=ghcr.io/spinkube/containerd-shim-spin/node-installer:dev make build-dev-installer-image
fi

echo "Loading node installer image into K3s..."
docker save ghcr.io/spinkube/containerd-shim-spin/node-installer:dev > node-installer.tar
sudo k3s ctr images import node-installer.tar
rm node-installer.tar

NODE_NAME=$(kubectl get nodes -o jsonpath='{.items[0].metadata.name}')
cp kwasm-job.yml k3s-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm/k3s-provision-kwasm/g" k3s-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm-dev/k3s-provision-kwasm-dev/g" k3s-kwasm-job.yml
sed -i "s/spin-test-control-plane/${NODE_NAME}/g" k3s-kwasm-job.yml

echo "Applying KWasm node installer job..."
kubectl apply -f ./k3s-kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl wait -n kwasm --for=condition=Ready pod --selector=job-name=k3s-provision-kwasm --timeout=90s || true
kubectl wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=k3s-provision-kwasm --timeout=60s

if ! kubectl get pods -n kwasm | grep -q "k3s-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl logs -n kwasm $(kubectl get pods -n kwasm -o name | grep k3s-provision-kwasm)
  exit 1
fi

echo "=== Step 4: Apply the workload ==="
kubectl apply -f ../deployments/workloads/workload.yaml

echo "Waiting for deployment to be ready..."
kubectl wait --for=condition=Available deployment/wasm-spin --timeout=120s

echo "Checking pod status..."
kubectl get pods

echo "=== Step 5: Test the workload ==="
echo "Waiting for service to be ready..."
sleep 10

echo "Testing workload with curl..."
kubectl port-forward svc/wasm-spin 8888:80 &
FORWARD_PID=$!
sleep 5

MAX_RETRIES=3
RETRY_COUNT=0
SUCCESS=false

while [ $RETRY_COUNT -lt $MAX_RETRIES ] && [ "$SUCCESS" = false ]; do
  if curl -s http://localhost:8888/hello | grep -q "Hello world from Spin!"; then
    SUCCESS=true
    echo "Workload test successful!"
  else
    echo "Retrying in 3 seconds..."
    sleep 3
    RETRY_COUNT=$((RETRY_COUNT+1))
  fi
done

kill $FORWARD_PID || true

if [ "$SUCCESS" = true ]; then
  echo "=== Integration Test Passed! ==="
  sudo /usr/local/bin/k3s-uninstall.sh 
  sudo rm -rf /etc/rancher/k3s
  sudo rm -rf /var/lib/rancher/k3s
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl describe pods
  kubectl logs $(kubectl get pods -o name | grep wasm-spin)
  sudo /usr/local/bin/k3s-uninstall.sh 
  sudo rm -rf /etc/rancher/k3s
  sudo rm -rf /var/lib/rancher/k3s
  exit 1
fi 