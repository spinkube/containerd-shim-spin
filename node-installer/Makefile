SPIN_VERSION = v2
IMAGE_NAME ?= ghcr.io/spinkube/containerd-shim-spin/node-installer
PLATFORM ?= linux/amd64
ARCH ?= x86_64
TARGET ?= $(ARCH)-unknown-linux-musl

compile-musl:
	make build-spin-cross-$(TARGET) -C ../

move-musl-to-tmp: compile-musl
	mkdir -p ./.tmp
	cp ../../containerd-shim-spin/target/$(TARGET)/release/containerd-shim-spin-$(SPIN_VERSION) ./.tmp/

build-multi-installer-image: move-musl-to-tmp
	docker buildx build -t $(IMAGE_NAME) --platform linux/amd64,linux/arm64 .

build-dev-installer-image: move-musl-to-tmp
	docker buildx build -t $(IMAGE_NAME) --load --platform $(PLATFORM) .
