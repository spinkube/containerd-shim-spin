#!/bin/bash

set -euo pipefail

DOCKER_IMAGES=("spin" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")
OUT_DIRS=("test/out_spin" "test/out_spin_keyvalue" "test/out_spin_outbound_redis" "test/out_spin_multi_trigger_app" "test/out_spin_static_assets" "test/out_spin_mqtt_message_logger")
IMAGES=("spin-hello-world" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")

# start a local registry at localhost:5000
docker run -d -p 5000:5000 --name test-registry registry:2

build_and_push() {
  local i=$1
  spin build -f "./images/${DOCKER_IMAGES[$i]}/spin.toml"
  if [ "${DOCKER_IMAGES[$i]}" == "spin-static-assets" ]; then
    export SPIN_OCI_ARCHIVE_LAYERS=1
  fi
  spin registry push "localhost:5000/spin-registry-push/${IMAGES[$i]}:latest" -f "./images/${DOCKER_IMAGES[$i]}/spin.toml" -k
}

# Iterate through the Docker images and build them in parallel
for i in "${!DOCKER_IMAGES[@]}"; do
  build_and_push "$i" &
done

# Wait for all background jobs to finish
wait

sleep 5

echo ">>> images are ready"