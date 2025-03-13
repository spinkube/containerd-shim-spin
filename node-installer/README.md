This directory contains resources for a custom node-installer image
intended to be used in conjunction with the [Kwasm Operator](https://github.com/KWasm/kwasm-operator).

This version of the image only contains the containerd-shim-spin-v2 shim, as
opposed to the default [kwasm-node-installer image](https://github.com/KWasm/kwasm-node-installer)
which also bundles other shims.

The intention is for the [spinkube/runtime-class-manager](https://github.com/spinkube/runtime-class-manager)
project to handle this concern in the future.

## Integration Test

The `integration-test.sh` script is used to test the node-installer image.
It creates a kind cluster, applies the KWasm node installer job, and then tests
the workload.

```bash
./integration-test.sh
```

## Build the Image Locally

```bash
make build-dev-installer-image
```

