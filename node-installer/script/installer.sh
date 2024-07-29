#!/usr/bin/env sh
set -euo pipefail

# Based on https://github.com/KWasm/kwasm-node-installer/blob/main/script/installer.sh
# Distilled to only configuring the Spin shim

KWASM_DIR=/opt/kwasm

CONTAINERD_CONF=/etc/containerd/config.toml
IS_MICROK8S=false
IS_K3S=false
IS_RKE2_AGENT=false
IS_K0S_WORKER=false
if pgrep -f snap/microk8s > /dev/null; then
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
elif pgrep -f /var/lib/k0s/bin/kubelet > /dev/null; then
    IS_K0S_WORKER=true
    CONTAINERD_CONF=/etc/k0s/containerd.d/spin.toml
    touch $NODE_ROOT$CONTAINERD_CONF
fi

mkdir -p $NODE_ROOT$KWASM_DIR/bin/

cp /assets/containerd-shim-spin-v2 $NODE_ROOT$KWASM_DIR/bin/

if ! grep -q spin $NODE_ROOT$CONTAINERD_CONF; then
    echo '
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-spin-v2"
' >> $NODE_ROOT$CONTAINERD_CONF
    rm -Rf $NODE_ROOT$KWASM_DIR/active
fi

if [ ! -f $NODE_ROOT$KWASM_DIR/active ]; then
    touch $NODE_ROOT$KWASM_DIR/active
    if $IS_MICROK8S; then
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart snap.microk8s.daemon-containerd
    elif $IS_K3S; then
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart k3s
    elif $IS_RKE2_AGENT; then
        nsenter --target 1 --mount --uts --ipc --net -- systemctl restart rke2-agent
    elif $IS_K0S_WORKER; then
        svc_name=k0sworker
        if ! systemctl list-units | grep -q $svc_name; then
            svc_name=k0scontroller
        fi

        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart $svc_name
    elif ls $NODE_ROOT/etc/init.d/containerd > /dev/null 2>&1 ; then
        nsenter --target 1 --mount --uts --ipc --net -- /etc/init.d/containerd restart
    else
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart containerd
    fi
else
    echo "No change in containerd/config.toml"
fi
