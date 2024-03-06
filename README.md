# Containerd Shim Spin

This project aims to provide the containerd shim implementation for [Spin](https://developer.fermyon.com/spin), which enables running Spin workloads on Kubernetes via [runwasi](https://github.com/deislabs/runwasi). This means that by installing this shim onto Kubernetes nodes, we can add a [runtime class](https://kubernetes.io/docs/concepts/containers/runtime-class/) to Kubernetes and schedule Spin workloads on those nodes. Your Spin apps can act just like container workloads!

[runwasi](https://github.com/deislabs/runwasi) is a project that aims to run WASI workloads managed by [containerd](https://containerd.io/).
