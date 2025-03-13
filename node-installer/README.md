This directory contains resources for a custom node-installer image
intended to be used in conjunction with the [Kwasm Operator](https://github.com/KWasm/kwasm-operator).

This version of the image only contains the containerd-shim-spin-v2 shim, as
opposed to the default [kwasm-node-installer image](https://github.com/KWasm/kwasm-node-installer)
which also bundles other shims.

The intention is for the [spinkube/runtime-class-manager](https://github.com/spinkube/runtime-class-manager)
project to handle this concern in the future.

## Integration Tests

The project includes integration test scripts for different Kubernetes distributions:

1. Kind: `make test-kind`
2. MiniKube: `make test-minikube`
3. MicroK8s: `make test-microk8s`
4. K3s: `make test-k3s`

## Build the Image Locally

```bash
make build-dev-installer-image
```

