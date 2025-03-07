RELEASE_FLAG ?= --release
PREFIX ?= /usr/local
INSTALL ?= install
ARCH ?= x86_64
TARGET ?= $(ARCH)-unknown-linux-musl
CONTAINERD_NAMESPACE ?= default
TEST_IMG_NAME ?= wasmtest_spin:latest
CTR_FLAGS ?=
ifeq ($(VERBOSE),)
VERBOSE_FLAG :=
else
VERBOSE_FLAG := -vvv
endif

IS_CI ?= false
BIN_DIR ?= 

UNAME_S := $(shell uname -s)

export


# build

.PHONY: build build-cargo build-cross-%

# build defaults to the static build using cross
# example. ARCH=x86_64 make build
build: build-cross-$(TARGET)
	echo "Build complete"

# build-cargo can be used to build binary targeting the host
build-cargo:
	cargo build $(RELEASE_FLAG) -p containerd-shim-spin-v2 $(VERBOSE_FLAG)

# build-cross can be be used to build any cross supported target (make build-cross-x86_64-unknown-linux-musl)
# example: make build-cross-x86_64-unknown-linux-musl
build-cross-%: install-cross
	cross build $(RELEASE_FLAG) --target $* -p containerd-shim-spin-v2 $(VERBOSE_FLAG)


# test

.PHONY: test unit-tests integration-tests integration-docker-build-push-tests integration-spin-registry-push-tests tests/collect-debug-logs

test: unit-tests integration-tests

# unit-tests
unit-tests: build
	cross test $(RELEASE_FLAG) --target $(TARGET) -p containerd-shim-spin-v2 $(VERBOSE_FLAG)

# integration-tests
integration-tests: prepare-cluster-and-images integration-docker-build-push-tests integration-spin-registry-push-tests
	echo "Integration tests complete. You may run 'make tests/clean' to clean up the test environment."

free-disk:
	./scripts/free-disk.sh

# integration-tests for workloads pushed using docker build push
integration-docker-build-push-tests:
	./scripts/run-integration-tests.sh "workloads-pushed-using-docker-build-push"

# integration-tests for workloads pushed using spin registry push
integration-spin-registry-push-tests:
	./scripts/run-integration-tests.sh "workloads-pushed-using-spin-registry-push"

tests/collect-debug-logs:
	./scripts/collect-debug-logs.sh 2>&1

# fmt

.PHONY: fmt fix
fmt:
	cargo +nightly fmt --all -- --check
	cargo clippy --all-targets --all-features --workspace -- --deny=warnings

fix:
	cargo +nightly fmt --all
	cargo clippy --all-targets --all-features --workspace --fix -- --deny=warnings


# install

.PHONY: install
install: build-cargo
	sudo $(INSTALL) ./target/release/containerd-shim-* $(PREFIX)/bin

# load

.PHONY: load load-%
load: load-spin

load-%: test/out_%/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import $<

test/out_%/img.tar: images/%/Dockerfile
	mkdir -p $(@D)
	# We disable provenance due to https://github.com/moby/buildkit/issues/3891.
	# A workaround for this (https://github.com/moby/buildkit/pull/3983) has been released in
	# buildkit v0.12.0. We can get rid of this flag with more recent versions of Docker that
	# bump buildkit.
	docker buildx build --provenance=false --platform=wasi/wasm --load -t $* ./images/$*
	docker save -o $@ $*

# run

.PHONY: run run-%
run: run-spin

run-%: install load
	sudo ctr -n $(CONTAINERD_NAMESPACE) run --rm --net-host $(CTR_FLAGS) --runtime=io.containerd.spin.v2 docker.io/library/$*:latest test$* "/"


# deploy

./PHONY: up move-bins deploy-workloads-pushed-using-docker-build-push deploy-workloads-pushed-using-spin-registry-push pod-terminates-test prepare-cluster-and-images

up:
	./scripts/up.sh

move-bins:
	./scripts/move-bins.sh $(BIN_DIR)

deploy-workloads-pushed-using-docker-build-push:
	./scripts/deploy-workloads.sh "workloads-pushed-using-docker-build-push"

deploy-workloads-pushed-using-spin-registry-push:
	./scripts/deploy-workloads.sh "workloads-pushed-using-spin-registry-push"

pod-terminates-test:
	./scripts/pod-terminates-test.sh

prepare-cluster-and-images: check-bins move-bins up free-disk pod-status-check

# clean

./PHONY: teardown-workloads tests/clean
teardown-workloads:
	./scripts/teardown-workloads.sh

tests/clean:
	./scripts/down.sh

# misc

# install cross
# pin cross to a specific commit to avoid breaking changes
.PHONY: install-cross install-k3d check-bins pod-status-check setup
install-cross:
	@if [ -z $$(which cross) ]; then RUSTFLAGS="-A warnings" cargo install cross --git https://github.com/cross-rs/cross --rev 49338b18fdb82dedb2a813664e2e565ca73e2047; fi
	@cross -V 2>/dev/null | grep 49338b1 || echo "WARN: unsupported version of cross found. Building containerd-shim-spin requires specific version of cross.\n\nPlease uninstall and run make install-cross to install the supported version."

install-k3d:
	wget -q -O - https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash

check-bins:
	./scripts/check-bins.sh

pod-status-check:
	./scripts/pod-status-check.sh

setup:
ifeq ($(UNAME_S),Linux)
	./scripts/setup-linux.sh
else
	@echo "Unsupported OS. Please use a Linux-based OS."
endif
