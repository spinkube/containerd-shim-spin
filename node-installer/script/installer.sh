#!/usr/bin/env sh
set -euo pipefail

# Based on: https://github.com/KWasm/kwasm-node-installer/blob/main/script/installer.sh

KWASM_DIR=/opt/kwasm

CONTAINERD_CONF=/etc/containerd/config.toml
IS_MICROK8S=false
IS_K3S=false
IS_RKE2_AGENT=false
if ps aux | grep kubelet | grep -q snap/microk8s; then
    CONTAINERD_CONF=/var/snap/microk8s/current/args/containerd-template.toml
    IS_MICROK8S=true
    if nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- ls /var/snap/microk8s/current/args/containerd-template.toml > /dev/null 2>&1 ;then
        KWASM_DIR=/var/snap/microk8s/common/kwasm
    else
        echo "Installer seems to run on microk8s but 'containerd-template.toml' not found."
        exit 1
    fi
elif ls $NODE_ROOT/var/lib/rancher/rke2/agent/etc/containerd/config.toml > /dev/null 2>&1 ; then
    IS_RKE2_AGENT=true
    cp $NODE_ROOT/var/lib/rancher/rke2/agent/etc/containerd/config.toml $NODE_ROOT/var/lib/rancher/rke2/agent/etc/containerd/config.toml.tmpl
    CONTAINERD_CONF=/var/lib/rancher/rke2/agent/etc/containerd/config.toml.tmpl
elif ls $NODE_ROOT/var/lib/rancher/k3s/agent/etc/containerd/config.toml > /dev/null 2>&1 ; then
    IS_K3S=true
    cp $NODE_ROOT/var/lib/rancher/k3s/agent/etc/containerd/config.toml $NODE_ROOT/var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl
    CONTAINERD_CONF=/var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl
fi

mkdir -p $NODE_ROOT$KWASM_DIR/bin/

cp /assets/containerd-shim-* $NODE_ROOT$KWASM_DIR/bin/

# TODO check if runtime config is already present
if ! grep -q wasmtime $NODE_ROOT$CONTAINERD_CONF; then
    echo '
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-spin-v2"
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.wasmedge]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-wasmedge-v1"
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.wasmer]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-wasmer-v1"
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.wasmtime]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-wasmtime-v1"
' >> $NODE_ROOT$CONTAINERD_CONF
    rm -Rf $NODE_ROOT$KWASM_DIR/active
fi

if [ ! -f $NODE_ROOT$KWASM_DIR/active ]; then
    touch $NODE_ROOT$KWASM_DIR/active
    if $IS_MICROK8S; then
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart snap.microk8s.daemon-containerd
    elif ls $NODE_ROOT/etc/init.d/containerd > /dev/null 2>&1 ; then
        nsenter --target 1 --mount --uts --ipc --net -- /etc/init.d/containerd restart
    elif ls $NODE_ROOT/etc/init.d/k3s > /dev/null 2>&1 ; then
        nsenter --target 1 --mount --uts --ipc --net -- /etc/init.d/k3s restart
    elif $IS_RKE2_AGENT; then
        nsenter --target 1 --mount --uts --ipc --net -- /bin/systemctl restart rke2-agent
    else
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- /bin/systemctl restart containerd
    fi
else
    echo "No change in containerd/config.toml"
fi
