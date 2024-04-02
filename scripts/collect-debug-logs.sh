#!/bin/bash

set -euo pipefail


echo "collecting debug info from CI run in 'debug-logs' dir"

mkdir -p debug-logs

echo "-> k3d cluster list" > debug-logs/kubernetes.log
k3d cluster list >> debug-logs/kubernetes.log
echo "" >> debug-logs/kubernetes.log

echo "-> kubectl get pods -n default -o wide" >> debug-logs/kubernetes.log
kubectl get pods -n default -o wide >> debug-logs/kubernetes.log
echo "" >> debug-logs/kubernetes.log

echo "-> kubectl describe pods -n default" >> debug-logs/kubernetes.log
kubectl describe pods -n default >> debug-logs/kubernetes.log
echo "" >> debug-logs/kubernetes.log

for node in `k3d node list --no-headers | awk '{print $1}'`; do
	echo "collecting containerd logs from $node"
	docker cp $node:/var/lib/rancher/k3s/agent/containerd/containerd.log debug-logs/$node.containerd.log || echo "containerd.log file not found in $node"
done