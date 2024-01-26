# Containerd Shim Spin

This project aims to provide containerd shim implementations that can run [Wasm](https://webassembly.org/) / [WASI](https://github.com/WebAssembly/WASI) workloads using [runwasi](https://github.com/deislabs/runwasi) as a library. This means that by installing these shims onto Kubernetes nodes, we can add a [runtime class](https://kubernetes.io/docs/concepts/containers/runtime-class/) to Kubernetes and schedule Wasm workloads on those nodes. Your Wasm pods and deployments can act just like container workloads!

[runwasi](https://github.com/deislabs/runwasi) is a project that aims to run WASI workloads managed by [containerd](https://containerd.io/).

