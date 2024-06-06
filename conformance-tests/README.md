# Spin Conformance Tests

A set of Spin [conformance tests](https://github.com/fermyon/conformance-tests) that ensures that this shim is a compliant runtime.

## Prerequisites
The following must be installed:
- `containerd`
- `ctr`
- `containerd-shim-spin`

Containerd must be configured to access the `containerd-shim-spin`:

1. Build the shim and add it to `$PATH`:
    ```sh
    cargo build --release
    sudo cp target/release/containerd-shim-spin-v2 /usr/bin/
    ```
2. Configure containerd to add the Spin shim as a runtime by adding the following to `/etc/containerd/config.toml`:
    ```toml
    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
        runtime_type = "io.containerd.spin.v2"
    ```
3. Restart containerd if it is running as a service
    ```sh
    systemctl restart containerd
    ```

## Running tests
Containerd can only be executed as a root user. Choose one of the following options
1. Build the `conformance-tests` bin and excute it as sudo user:
    ```sh
    cargo build 
    sudo ../target/debug/conformance-tests
    ```
2. Run cargo as root by passing in the environment from the user's context and full path to cargo:
    ```sh
    sudo -E $HOME/.cargo/bin/cargo run
    ```
3. Follow the [containerd instructions](https://github.com/containerd/containerd/blob/main/docs/rootless.md) to configure it to be runnable as a non-root user