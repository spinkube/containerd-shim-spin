#!/bin/bash

set -euo pipefail

## Deploy workloads into k3d cluster
if [ "$1" == "workloads-pushed-using-spin-registry-push" ]; then
    make deploy-workloads-pushed-using-spin-registry-push
else
    make deploy-workloads-pushed-using-docker-build-push
fi

## Verify pods can be terminated successfully
make pod-terminates-test
	
## Run integration tests
cargo test -p containerd-shim-spin-tests -- --nocapture

## tests done, cleanup workloads for next test
make teardown-workloads