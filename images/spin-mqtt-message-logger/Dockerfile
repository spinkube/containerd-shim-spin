FROM --platform=${BUILDPLATFORM} rust:1.85 AS build
WORKDIR /opt/build
COPY . .
RUN rustup target add wasm32-wasip1 && cargo build --target wasm32-wasip1 --release

FROM scratch
COPY --from=build /opt/build/target/wasm32-wasip1/release/mqtt_message_logger.wasm ./target/wasm32-wasip1/release/mqtt_message_logger.wasm
COPY --from=build /opt/build/spin.toml .