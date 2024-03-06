# Based on: https://github.com/KWasm/kwasm-node-installer/blob/main/images/installer/Dockerfile

FROM scratch AS bin
ARG TARGETPLATFORM
COPY ./.tmp/${TARGETPLATFORM} /

FROM ubuntu:22.04 AS download-containerd-runwasi
RUN DEBIAN_FRONTEND=noninteractive apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y curl

RUN mkdir -p /release/bin/ \
    && curl -L https://github.com/containerd/runwasi/releases/download/containerd-shim-wasmedge%2Fv0.3.0/containerd-shim-wasmedge-$(uname -m | sed s/arm64/aarch64/g | sed s/amd64/x86_64/g).tar.gz | tar -xzf - -C /release/bin/ \
    && curl -L https://github.com/containerd/runwasi/releases/download/containerd-shim-wasmtime%2Fv0.3.0/containerd-shim-wasmtime-$(uname -m | sed s/arm64/aarch64/g | sed s/amd64/x86_64/g).tar.gz | tar -xzf - -C /release/bin/ \
    && curl -L https://github.com/containerd/runwasi/releases/download/containerd-shim-wasmer%2Fv0.3.0/containerd-shim-wasmer-$(uname -m | sed s/arm64/aarch64/g | sed s/amd64/x86_64/g).tar.gz | tar -xzf - -C /release/bin/

FROM busybox

COPY script/installer.sh /script/installer.sh
COPY --link --from=bin / /assets
COPY --link --from=download-containerd-runwasi /release/bin/containerd-shim-wasmedge-v1 /assets/
COPY --link --from=download-containerd-runwasi /release/bin/containerd-shim-wasmer-v1 /assets/
COPY --link --from=download-containerd-runwasi /release/bin/containerd-shim-wasmtime-v1 /assets/
CMD sh /script/installer.sh wasmedge
