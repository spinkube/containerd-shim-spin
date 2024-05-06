BUILD_TARGETS = build-spin-cross-$(TARGET)

PREFIX ?= /usr/local
INSTALL ?= install
TEST_IMG_NAME_spin ?= wasmtest_spin:latest
ARCH ?= x86_64
TARGET ?= $(ARCH)-unknown-linux-musl
PYTHON ?= python3
CONTAINERD_NAMESPACE ?= default
ifeq ($(VERBOSE),)
VERBOSE_FLAG :=
else
VERBOSE_FLAG := -vvv
endif

BIN_DIR ?= 

.PHONY: test
test: unit-tests integration-tests

.PHONY: unit-tests
unit-tests: build
	cross test --release --manifest-path=containerd-shim-spin/Cargo.toml --target $(TARGET)

.PHONY: check-bins
check-bins:
	./scripts/check-bins.sh

./PHONY: move-bins
move-bins:
	./scripts/move-bins.sh $(BIN_DIR)

./PHONY: up
up:
	./scripts/up.sh

./PHONY: pod-status-check
pod-status-check:
	./scripts/pod-status-check.sh

./PHONY: deploy-workloads-pushed-using-docker-build-push
deploy-workloads-pushed-using-docker-build-push:
	./scripts/deploy-workloads.sh "workloads-pushed-using-docker-build-push"

./PHONY: deploy-workloads-pushed-using-spin-registry-push
deploy-workloads-pushed-using-spin-registry-push:
	./scripts/deploy-workloads.sh "workloads-pushed-using-spin-registry-push"

./PHONY: pod-terminates-test
pod-terminates-test:
	./scripts/pod-terminates-test.sh

.PHONY: integration-tests
integration-tests: prepare-cluster-and-images integration-docker-build-push-tests integration-spin-registry-push-tests

.PHONY: integration-docker-build-push-tests
integration-docker-build-push-tests:
	./scripts/run-integration-tests.sh "workloads-pushed-using-docker-build-push"

.PHONY: integration-spin-registry-push-tests pod-terminates-test
integration-spin-registry-push-tests:
	./scripts/run-integration-tests.sh "workloads-pushed-using-spin-registry-push"

.PHONY: prepare-cluster-and-images
prepare-cluster-and-images: check-bins move-bins up pod-status-check
.PHONY: tests/collect-debug-logs
tests/collect-debug-logs:
	./scripts/collect-debug-logs.sh 2>&1

.PHONY: tests/clean
tests/clean:
	./scripts/down.sh

.PHONY: fmt
fmt:
	cargo +nightly fmt --all -- --check
	cargo clippy --all-targets --all-features --workspace -- --deny=warnings

.PHONY: fix
fix:
	cargo +nightly fmt --all
	cargo clippy --all-targets --all-features --workspace --fix -- --deny=warnings

.PHONY: build
build: build-spin-cross-$(TARGET)
	echo "Build complete"

# pin cross to a specific commit to avoid breaking changes
.PHONY: install-cross
install-cross:
	@if [ -z $$(which cross) ]; then cargo install cross --git https://github.com/cross-rs/cross --rev 5896ed1359642510855ca9ee50ce7fdf75c50e3c; fi

# build-cross can be be used to build any cross supported target (make build-cross-x86_64-unknown-linux-musl)
.PHONY: $(BUILD_TARGETS)
$(BUILD_TARGETS): SHIM = $(word 2,$(subst -, ,$@))
$(BUILD_TARGETS): install-cross
	cross build --release --target $(TARGET) --manifest-path=containerd-shim-$(SHIM)/Cargo.toml $(VERBOSE_FLAG)

.PHONY: build-%
build-%:
	cargo build --release --manifest-path=containerd-shim-$*/Cargo.toml

.PHONY: install
install: build-spin
	sudo $(INSTALL) ./target/release/containerd-shim-* $(PREFIX)/bin

.PHONY: update-deps
update-deps:
	cargo update

test/out_%/img.tar: images/%/Dockerfile
	mkdir -p $(@D)
	# We disable provenance due to https://github.com/moby/buildkit/issues/3891.
	# A workaround for this (https://github.com/moby/buildkit/pull/3983) has been released in
	# buildkit v0.12.0. We can get rid of this flag with more recent versions of Docker that
	# bump buildkit.
	docker buildx build --provenance=false --platform=wasi/wasm --load -t $(TEST_IMG_NAME_$*) ./images/$*
	docker save -o $@ $(TEST_IMG_NAME_$*)

load: test/out_spin/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import test/out_spin/img.tar

.PHONY: run_%
run_%: install load
	sudo ctr run --net-host --rm --runtime=io.containerd.$*.v1 docker.io/library/$(TEST_IMG_NAME_$*) test$*

.PHONY: clean
clean: clean-spin
	test -f $(PREFIX)/bin/containerd-shim-spin-* && sudo rm -rf $(PREFIX)/bin/containerd-shim-$(proj)-* || true;
	test -d ./test && sudo rm -rf ./test || true

.PHONY: clean-%
clean-%:
	cargo clean --manifest-path containerd-shim-$*/Cargo.toml