# Containerd Shim Spin

This project aims to provide the [containerd shim](https://github.com/containerd/containerd/blob/main/core/runtime/v2/README.md#runtime-shim) implementation for [Spin](https://developer.fermyon.com/spin), which enables running Spin workloads on Kubernetes via [runwasi](https://github.com/deislabs/runwasi). This means that by installing this shim onto Kubernetes nodes, we can add a [runtime class](https://kubernetes.io/docs/concepts/containers/runtime-class/) to Kubernetes and schedule Spin workloads on those nodes. Your Spin apps can act just like container workloads!

[runwasi](https://github.com/deislabs/runwasi) is a project that aims to run WASI workloads managed by [containerd](https://containerd.io/).

## Table of Contents

- [Shim and Spin Version Map](#shim-and-spin-version-map)
- [Documentation](#documentation)
- [Building and running the shim on host](#building-and-running-the-containerd-shim-spin-on-host)
- [Installing the shim on Kubernetes Nodes](#installing-the-containerd-shim-spin-on-kubernetes-nodes)
- [Locating build artifacts](#locating-build-artifacts)
- [Feedback](#feedback)
- [Contributing](#contributing)

## Shim and Spin Version Map

Below is a table for referencing the version of the Spin runtime used in each `containerd-shim-spin` release.

| **shim version** | v0.12.0                                                       | v0.13.0                                                       | v0.14.0                                                       | v0.14.1                                                       | v0.15.0                                                       | v0.15.1                                                       | v0.16.0    | v0.17.0    | v0.18.0 |
| ---------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | --- | --- | --- |
| **spin**         | [v2.2.0](https://github.com/fermyon/spin/releases/tag/v2.2.0) | [v2.3.1](https://github.com/fermyon/spin/releases/tag/v2.3.1) | [v2.4.2](https://github.com/fermyon/spin/releases/tag/v2.4.2) | [v2.4.3](https://github.com/fermyon/spin/releases/tag/v2.4.3) | [v2.6.0](https://github.com/fermyon/spin/releases/tag/v2.6.0) | [v2.6.0](https://github.com/fermyon/spin/releases/tag/v2.6.0) | [v2.6.0](https://github.com/fermyon/spin/releases/tag/v2.6.0) | [v3.0.0](https://github.com/fermyon/spin/releases/tag/v3.0.0) | [v3.1.2](https://github.com/fermyon/spin/releases/tag/v3.1.2) |

## Documentation

To learn more about the Containerd Shim Spin, please visit [the official Containerd Shim Spin documentation](https://www.spinkube.dev/docs/topics/architecture/#containerd-shim-spin).

## Building and running the `containerd-shim-spin` on host

Make sure you have installed dependencies:
```bash
make setup # setup linux environment
```

Build, install, and run the shim binary:

```bash
make run # run the shim binary
```

You may open another terminal and run the following command to test the shim:
```bash
curl 0.0.0.0:80/hello
```

## Installing the `containerd-shim-spin` on Kubernetes Nodes

In order to run Spin applications on your cluster, you must complete the following three steps:

1. Install the shim on each Node that should support Spin apps
2. Update the containerd configuration to recognize the shim
3. Apply the Kubernetes `RuntimeClass` for the shim

Repeating steps 1 and 2 for each node on a cluster can be a time-consuming and manual process. For this reason, SpinKube provides a [`runtime-class-manager`](https://www.spinkube.dev/docs/topics/architecture/#runtime-class-manager) (previously the `kwasm` operator) that enables you to skip over step 1 and 2. See the [SpinKube installation guide](https://www.spinkube.dev/docs/install/installing-with-helm/) for more information on installing with Helm.

To carry out the installation step-by-step, do the following:

1. Install the shim on each Node that should support Spin apps

    Install a [release of the shim](https://github.com/spinkube/containerd-shim-spin/releases) on the `PATH` of your Kubernetes worker nodes. For example, copy `containerd-shim-spin-v2` to `/bin`. Shims are additive, so once the `containerd-shim-spin` is installed on a Node, it can support Spin WebAssembly apps alongside Linux containers.

2. Add the following to the containerd `config.toml` that maps the runtime type to the shim binary from step 1.

    If you are using containerd 2 or later, add:

    ```toml
    [plugins."io.containerd.cri.v1.runtime".containerd.runtimes.spin]
      runtime_type = "io.containerd.spin.v2"
    [plugins."io.containerd.cri.v1.runtime".containerd.runtimes.spin.options]
      SystemdCgroup = true
    ```

    Otherwise, add:

    ```toml
    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
      runtime_type = "io.containerd.spin.v2"
    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin.options]
      SystemdCgroup = true
    ```

    The [Node Installer script](./node-installer/script/installer.sh) that is used by the [`runtime-class-manager`](https://www.spinkube.dev/docs/topics/architecture/#runtime-class-manager) does this for you and is a good reference to understand the common paths to the containerd configuration file for popular Kubernetes distributions.

3. Apply a runtime class that contains a handler that matches the "spin" config runtime name from step 2.

    This ensures that the image is executed with the correct runtime, namely the `containerd-shim-spin`.

    > Note: You likely want to customize the Runtime Class with a [`nodeSelector`](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#nodeselector) to ensure Pods are only scheduled to Nodes where the shim has been installed.

    ```yaml
    apiVersion: node.k8s.io/v1
    handler: spin
    kind: RuntimeClass
    metadata:
      name: wasmtime-spin-v2
    ```

4. Deploy a Spin app to your cluster with the specified `RuntimeClass` name matching the "wasmtime-spin-v2" runtime class from step 3. The [Spin Operator](../spin-operator/_index.md) does this for you, translating `SpinApp` custom resources into Kubernetes deployments.

    ```yaml
    apiVersion: apps/v1
    kind: Deployment
    metadata:
      name: wasm-spin
    spec:
      replicas: 1
      selector:
        matchLabels:
          app: wasm-spin
      template:
        metadata:
          labels:
            app: wasm-spin
        spec:
          runtimeClassName: wasmtime-spin-v2
          containers:
            - name: spin-hello
              image: ghcr.io/spinkube/containerd-shim-spin/examples/spin-rust-hello:v0.13.0
              command: ["/"]
    ```

## Locating build artifacts

### Versioned releases

A GitHub release is created for each versioned release with build artifacts attached, including shim binaries for all supported architectures. See the [Releases page](https://github.com/spinkube/containerd-shim-spin/releases) for the full listing.

In addition, container images for k3d, node-installer and example apps are published using the release version for their tags, eg [ghcr.io/spinkube/containerd-shim-spin:v0.15.1](https://github.com/spinkube/containerd-shim-spin/pkgs/container/containerd-shim-spin%2Fnode-installer/240852005?tag=v0.15.1). A listing of all images and their tags can be seen from the [packages page](https://github.com/orgs/spinkube/packages?repo_name=containerd-shim-spin).

### Canary builds

On every push to the `main` branch, the [release workflow](https://github.com/spinkube/containerd-shim-spin/actions/workflows/release.yaml) will run and attach shim binary artifacts once finished.

Container images for k3d, node-installer and example apps are also published, using a unique tag of the form `$(date +%Y%m%d-%H%M%S)-g$(git rev-parse --short HEAD)`. A listing of all images and their tags can be seen from the [packages page](https://github.com/orgs/spinkube/packages?repo_name=containerd-shim-spin).

### Branch builds

For every push to a feature branch with a corresponding PR, the [ci workflow](https://github.com/spinkube/containerd-shim-spin/actions/workflows/ci.yaml) will run and attach shim binary artifacts once finished.

If the feature branch is created from the origin repo, the [node-installer image](https://github.com/spinkube/containerd-shim-spin/pkgs/container/containerd-shim-spin%2Fnode-installer) will be published with a tag following the same convention as `main` builds. You can locate the tag corresponding to your branch via the commit string portion.

> Note: A node-installer image won't be pushed for feature branches on forks, as they don't have access to the credentials needed to publish to the ghcr.io container registry.

## Feedback

For questions or support, please visit our [Slack channel](https://cloud-native.slack.com/archives/C06PC7JA1EE): #spinkube. 

## Contributing

If you would like to contribute, please visit this [contributing](https://www.spinkube.dev/docs/contrib/) page.
