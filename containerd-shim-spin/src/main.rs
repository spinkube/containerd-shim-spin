use std::env;

use containerd_shim::Config;
use containerd_shim_wasm::{
    container::Instance,
    sandbox::cli::{revision, shim_main, version},
};

mod engine;
mod version;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--version" {
        version::print_version();
        return;
    }

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
