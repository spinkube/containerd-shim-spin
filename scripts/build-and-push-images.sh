#!/bin/bash

set -euo pipefail

RUN_SPIN=false
RUN_DOCKER=false

cluster_name="test-cluster"
OUT_DIRS=("test/out_spin" "test/out_spin_keyvalue" "test/out_spin_outbound_redis" "test/out_spin_multi_trigger_app" "test/out_spin_static_assets" "test/out_spin_mqtt_message_logger")
IMAGES=("spin-hello-world" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")


spin_build_and_push() {
  local i=$1
  spin build -f "./images/${IMAGES[$i]}/spin.toml"
  if [ "${IMAGES[$i]}" == "spin-static-assets" ]; then
    export SPIN_OCI_ARCHIVE_LAYERS=1
  fi
  spin registry push "localhost:5000/spin-registry-push/${IMAGES[$i]}:latest" -f "./images/${IMAGES[$i]}/spin.toml" -k
}

docker_build_and_push() {
  local image="$1"
  local out_dir="$2"
  
  docker buildx build -t "${image}:latest" "./images/${image}" --load
  mkdir -p "${out_dir}"
  docker save -o "${out_dir}/img.tar" "${image}:latest"
  k3d image import "${out_dir}/img.tar" -c "$cluster_name"
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --spin) RUN_SPIN=true ;;
    --docker) RUN_DOCKER=true ;;
    --both) RUN_SPIN=true; RUN_DOCKER=true ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
  shift
done

if ! $RUN_SPIN && ! $RUN_DOCKER; then
  echo "Error: At least one of --spin, --docker, or --both must be specified."
  exit 1
fi


if $RUN_SPIN; then
  echo "Running Spin builds and pushes..."
  if ! docker ps | grep -q test-registry; then
    docker run -d -p 5000:5000 --name test-registry registry:2
  fi
  for i in "${!IMAGES[@]}"; do
    spin_build_and_push "$i" &
  done
fi

if $RUN_DOCKER; then
  echo "Running Docker builds and pushes..."
  for i in "${!IMAGES[@]}"; do
    docker_build_and_push "${IMAGES[$i]}" "${OUT_DIRS[$i]}" &
  done
fi

# Wait for all background jobs to finish
wait

sleep 5
echo "Images are ready"