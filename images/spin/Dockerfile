FROM --platform=${BUILDPLATFORM} rust:1.85 AS build
WORKDIR /opt/build
COPY . .
RUN rustup target add wasm32-wasip1 && cargo build --target wasm32-wasip1 --release

FROM --platform=linux/amd64 golang:1.23.2-bullseye AS build-go
WORKDIR /opt/build
COPY . .
RUN curl -LO https://github.com/tinygo-org/tinygo/releases/download/v0.34.0/tinygo_0.34.0_amd64.deb && dpkg -i tinygo_0.34.0_amd64.deb
RUN cd go-hello && tinygo build -target=wasi -o spin_go_hello.wasm main.go

FROM scratch
COPY --from=build /opt/build/target/wasm32-wasip1/release/spin_rust_hello.wasm ./target/wasm32-wasip1/release/spin_rust_hello.wasm
COPY --from=build /opt/build/spin.toml .
COPY --from=build-go /opt/build/go-hello/spin_go_hello.wasm ./go-hello/spin_go_hello.wasm
