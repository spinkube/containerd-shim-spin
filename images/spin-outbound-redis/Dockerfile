FROM --platform=${BUILDPLATFORM} rust:1.85 AS build
WORKDIR /opt/build
RUN rustup target add wasm32-wasip1
COPY . .
RUN cargo build --target wasm32-wasip1 --release

FROM scratch
COPY --from=build /opt/build/target/wasm32-wasip1/release/spin_outbound_redis.wasm ./target/wasm32-wasip1/release/spin_outbound_redis.wasm
COPY --from=build /opt/build/spin.toml .