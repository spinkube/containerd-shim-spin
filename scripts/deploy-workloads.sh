#!/bin/bash

set -euo pipefail

# Check if kubectl is installed
if ! command -v kubectl &> /dev/null; then
    echo "kubectl is not installed. Installing..."
    curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
    sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl;
fi

update_mqtt_workload_with_broker_cluster_ip() {
    # The MQTT trigger cannot do DNS resolution, so we need to use the IP address of the MQTT broker
    # Replace "EMQX_CLUSTER_IP" with the actual ClusterIP of the EMQX service
    local cluster_ip=$(kubectl get svc emqx -o jsonpath='{.spec.clusterIP}')
    sed -i "s/EMQX_CLUSTER_IP/$cluster_ip/g" tests/workloads-pushed-using-spin-registry-push/workloads.yaml
    echo "Updated workloads.yaml with ClusterIP: $cluster_ip"
}


# apply the workloads
echo ">>> apply workloads"
kubectl apply -f tests/workloads-common

if [ "$1" == "workloads-pushed-using-spin-registry-push" ]; then
    update_mqtt_workload_with_broker_cluster_ip
    echo "deploying spin apps pushed to registry using 'spin registry push' command"
    kubectl apply -f tests/workloads-pushed-using-spin-registry-push
else
    echo "deploying spin apps pushed to registry using 'docker build && k3d image import' command"
    kubectl apply -f tests/workloads-pushed-using-docker-build-push
fi

# wait for all the pods to be ready
kubectl wait --for=condition=ready --timeout=50s pod --all

# get and describe all the pods
echo ">>> Pods:"
kubectl get pods -o wide
kubectl describe pods

# get and describe all the deployments
echo ">>> Deployments:"
kubectl get deployments -o wide
kubectl describe deployments

# get and describe all the services
echo ">>> Services:"
kubectl get services -o wide
kubectl describe services

