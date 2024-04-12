# Containerd Shim Spin

This project aims to provide the [containerd shim](https://github.com/containerd/containerd/blob/main/core/runtime/v2/README.md#runtime-shim) implementation for [Spin](https://developer.fermyon.com/spin), which enables running Spin workloads on Kubernetes via [runwasi](https://github.com/deislabs/runwasi). This means that by installing this shim onto Kubernetes nodes, we can add a [runtime class](https://kubernetes.io/docs/concepts/containers/runtime-class/) to Kubernetes and schedule Spin workloads on those nodes. Your Spin apps can act just like container workloads!

[runwasi](https://github.com/deislabs/runwasi) is a project that aims to run WASI workloads managed by [containerd](https://containerd.io/).

## Documentation

To learn more about the Containerd Shim Spin, please visit [the official Containerd Shim Spin documentation](https://www.spinkube.dev/docs/containerd-shim-spin/).

## Feedback

For questions or support, please visit our [Slack channel](https://cloud-native.slack.com/archives/C06PC7JA1EE): #spinkube. 

## Contributing

If you would like to contribute, please visit this [contributing](https://www.spinkube.dev/docs/containerd-shim-spin/contributing/) page.