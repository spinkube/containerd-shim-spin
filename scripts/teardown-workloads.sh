#!/bin/bash

set -euo pipefail

case "$1" in
  k3d)
    TESTS_PATH="tests/k3d"
    ;;
  k3s)
    TESTS_PATH="tests/k3s"
    ;;
  *)
    echo "Invalid argument. Use 'k3d' or 'k3s'."
    exit 1
    ;;
esac

kubectl delete -f "tests/workloads-common" --wait --timeout 60s --ignore-not-found=true
kubectl delete -f "tests/workloads-pushed-using-docker-build-push" --wait --timeout 60s --ignore-not-found=true
kubectl delete -f "$TESTS_PATH/workloads-pushed-using-spin-registry-push" --wait --timeout 60s --ignore-not-found=true
kubectl wait pod --for=delete -l app=wasm-spin -l app=spin-keyvalue -l app=spin-outbound-redis -l app=spin-multi-trigger-app --timeout 60s