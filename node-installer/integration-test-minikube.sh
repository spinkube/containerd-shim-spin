#!/bin/bash
set -euo pipefail

echo "=== Step 1: Create a MiniKube cluster ==="
minikube start -p spin-minikube --driver=docker --container-runtime=containerd

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

echo "Loading node installer image into MiniKube..."
minikube image load ghcr.io/spinkube/containerd-shim-spin/node-installer:dev -p spin-minikube

NODE_NAME=$(kubectl get nodes --context=minikube -o jsonpath='{.items[0].metadata.name}')
cp kwasm-job.yml minikube-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm/spin-minikube-provision-kwasm/g" minikube-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm-dev/spin-minikube-provision-kwasm-dev/g" minikube-kwasm-job.yml
sed -i "s/spin-test-control-plane/${NODE_NAME}/g" minikube-kwasm-job.yml

echo "Applying KWasm node installer job..."
kubectl apply -f ./minikube-kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl wait -n kwasm --for=condition=Ready pod --selector=job-name=spin-minikube-provision-kwasm --timeout=90s || true
kubectl wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=spin-minikube-provision-kwasm --timeout=60s

if ! kubectl get pods -n kwasm | grep -q "spin-minikube-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl logs -n kwasm $(kubectl get pods -n kwasm -o name | grep spin-minikube-provision-kwasm)
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
minikube service wasm-spin --url -p spin-minikube > service_url.txt
SERVICE_URL=$(cat service_url.txt)

MAX_RETRIES=3
RETRY_COUNT=0
SUCCESS=false

while [ $RETRY_COUNT -lt $MAX_RETRIES ] && [ "$SUCCESS" = false ]; do
  if curl -s $SERVICE_URL/hello | grep -q "Hello world from Spin!"; then
    SUCCESS=true
    echo "Workload test successful!"
  else
    echo "Retrying in 3 seconds..."
    sleep 3
    RETRY_COUNT=$((RETRY_COUNT+1))
  fi
done

if [ "$SUCCESS" = true ]; then
  echo "=== Integration Test Passed! ==="
  minikube delete -p spin-minikube
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl describe pods
  kubectl logs $(kubectl get pods -o name | grep wasm-spin)
  minikube delete -p spin-minikube
  exit 1
fi 