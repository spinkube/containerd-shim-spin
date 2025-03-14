#!/bin/bash
set -euo pipefail

echo "=== Step 1: Create a kind cluster ==="
if kind get clusters | grep -q "spin-test"; then
  echo "Deleting existing cluster..."
  kind delete cluster --name spin-test
fi

echo "Creating kind cluster..."
kind create cluster --config .kind/kind-config.yaml
kubectl --context=kind-spin-test wait --for=condition=Ready nodes --all --timeout=90s

echo "=== Step 2: Create namespace and deploy RuntimeClass ==="
kubectl --context=kind-spin-test create namespace kwasm || true
kubectl --context=kind-spin-test apply -f ../deployments/workloads/runtime.yaml

echo "=== Step 3: Build and deploy the KWasm node installer ==="
if ! docker image inspect ghcr.io/spinkube/containerd-shim-spin/node-installer:v0.18.0 >/dev/null 2>&1; then
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

echo "Loading node installer image into kind..."
kind load docker-image ghcr.io/spinkube/containerd-shim-spin/node-installer:dev --name spin-test

echo "Applying KWasm node installer job..."
kubectl --context=kind-spin-test apply -f ./kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl --context=kind-spin-test wait -n kwasm --for=condition=Ready pod --selector=job-name=spin-test-control-plane-provision-kwasm --timeout=90s || true
kubectl --context=kind-spin-test wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=spin-test-control-plane-provision-kwasm --timeout=60s

if ! kubectl --context=kind-spin-test get pods -n kwasm | grep -q "spin-test-control-plane-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl --context=kind-spin-test logs -n kwasm $(kubectl --context=kind-spin-test get pods -n kwasm -o name | grep spin-test-control-plane-provision-kwasm)
  exit 1
fi

echo "=== Step 4: Apply the workload ==="
kubectl --context=kind-spin-test apply -f ../deployments/workloads/workload.yaml

echo "Waiting for deployment to be ready..."
kubectl --context=kind-spin-test wait --for=condition=Available deployment/wasm-spin --timeout=120s

echo "Checking pod status..."
kubectl --context=kind-spin-test get pods

echo "=== Step 5: Test the workload ==="
echo "Waiting for service to be ready..."
sleep 10

echo "Testing workload with curl..."
kubectl --context=kind-spin-test port-forward svc/wasm-spin 8888:80 &
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

kill $FORWARD_PID

if [ "$SUCCESS" = true ]; then
  echo "=== Integration Test Passed! ==="
  kind delete cluster --name spin-test
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl --context=kind-spin-test describe pods
  kubectl --context=kind-spin-test logs $(kubectl --context=kind-spin-test get pods -o name | grep wasm-spin)
  kind delete cluster --name spin-test
  exit 1
fi 