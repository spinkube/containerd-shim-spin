#!/bin/bash

set -euo pipefail

## test that the workload pods can be terminated
kubectl delete pod -l app=wasm-spin --timeout 20s
kubectl delete pod -l app=spin-keyvalue --timeout 20s
kubectl delete pod -l app=spin-outbound-redis --timeout 20s
kubectl delete pod -l app=spin-multi-trigger-app --timeout 20s
kubectl delete pod -l app=spin-mqtt-message-logger --timeout 20s

