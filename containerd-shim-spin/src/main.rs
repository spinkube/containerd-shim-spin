use containerd_shim::Config;
use containerd_shim_wasm::{
    container::Instance,
    sandbox::cli::{revision, shim_main, version},
};

mod constants;
mod engine;
mod source;
mod trigger;
mod utils;

fn main() {
    // Configure the shim to have only error level logging for performance improvements.
    let shim_config = Config {
        default_log_level: "error".to_string(),
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
