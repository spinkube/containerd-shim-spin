SPIN_VERSION = v2
IMAGE_NAME ?= ghcr.io/spinkube/containerd-shim-spin/node-installer
PLATFORM ?= linux/amd64
ARCH ?= x86_64
TARGET ?= $(ARCH)-unknown-linux-musl
BINARY := ./.tmp/$(PLATFORM)/containerd-shim-spin-$(SPIN_VERSION)

compile-musl:
	make build-cross-$(TARGET) -C ../

$(BINARY): 
	mkdir -p ./.tmp/$(PLATFORM)
	$(MAKE) compile-musl
	cp ../target/$(TARGET)/release/containerd-shim-spin-$(SPIN_VERSION) $(BINARY)

build-multi-installer-image: $(BINARY)
	docker buildx build -t $(IMAGE_NAME) --platform linux/amd64,linux/arm64 .

build-dev-installer-image: $(BINARY)
	docker buildx build -t $(IMAGE_NAME) --load --platform $(PLATFORM) .

test:
	./integration-test.sh
