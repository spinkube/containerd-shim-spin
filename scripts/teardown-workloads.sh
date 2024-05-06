#!/bin/bash

set -euo pipefail

kubectl delete -f tests/workloads-common --wait --timeout 60s --ignore-not-found=true
kubectl delete -f tests/workloads-pushed-using-docker-build-push --wait --timeout 60s --ignore-not-found=true
kubectl delete -f tests/workloads-pushed-using-spin-registry-push --wait --timeout 60s --ignore-not-found=true
kubectl wait pod --for=delete -l app=wasm-spin -l app=spin-keyvalue -l app=spin-outbound-redis -l app=spin-multi-trigger-app --timeout 60s