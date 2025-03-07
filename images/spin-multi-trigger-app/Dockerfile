FROM --platform=${BUILDPLATFORM} rust:1.85 AS build
WORKDIR /opt/build
RUN rustup target add wasm32-wasip1
COPY . .

WORKDIR /opt/build/spin-http-trigger
RUN cargo build --target wasm32-wasip1 --release

WORKDIR /opt/build/spin-redis-trigger
RUN cargo build --target wasm32-wasip1 --release

FROM scratch
COPY --from=build /opt/build/spin-http-trigger/target/wasm32-wasip1/release/spin_http_trigger.wasm spin-http-trigger/target/wasm32-wasip1/release/spin_http_trigger.wasm
COPY --from=build /opt/build/spin-redis-trigger/target/wasm32-wasip1/release/spin_redis_trigger.wasm spin-redis-trigger/target/wasm32-wasip1/release/spin_redis_trigger.wasm
COPY --from=build /opt/build/spin.toml .