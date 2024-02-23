use containerd_shim::Config;
use containerd_shim_wasm::container::Instance;
use containerd_shim_wasm::sandbox::cli::{revision, shim_main, version};

mod engine;

fn main() {
    // Configure the shim to disable all logging for performance improvements.
    // TODO: consider supporting some logging once log level specification is
    // supported in https://github.com/containerd/rust-extensions/pull/247
    let shim_config = Config {
        no_setup_logger: true,
        ..Default::default()
    };
    shim_main::<Instance<engine::SpinEngine>>(
        "spin",
        version!(),
        revision!(),
        "v2",
        Some(shim_config),
    );
}
