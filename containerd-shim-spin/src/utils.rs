use std::{
    env,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use spin_app::locked::LockedApp;
use spin_loader::cache::Cache;

use crate::constants;

// create a cache directory at /.cache
// this is needed for the spin LocalLoader to work
// TODO: spin should provide a more flexible `loader::from_file` that
// does not assume the existence of a cache directory
pub(crate) async fn initialize_cache() -> Result<Cache, anyhow::Error> {
    let cache_dir = PathBuf::from("/.cache");
    let cache = Cache::new(Some(cache_dir.clone()))
        .await
        .context("failed to create cache")?;
    env::set_var("XDG_CACHE_HOME", &cache_dir);
    Ok(cache)
}

pub(crate) async fn handle_archive_layer(
    cache: &Cache,
    bytes: impl AsRef<[u8]>,
    digest: impl AsRef<str>,
) -> Result<()> {
    // spin_oci::client::unpack_archive_layer attempts to create a tempdir via tempfile::tempdir()
    // which will fall back to /tmp if TMPDIR is not set. /tmp is either not found or not accessible
    // in the shim environment, hence setting to current working directory.
    if env::var("TMPDIR").is_err() {
        log::debug!(
            "<<< TMPDIR is not set; setting to current working directory for unpacking archive layer"
        );
        env::set_var("TMPDIR", env::current_dir().unwrap_or(".".into()));
    }

    spin_oci::client::unpack_archive_layer(cache, bytes, digest).await
}

pub(crate) fn parse_addr(addr: &str) -> Result<SocketAddr> {
    let addrs: SocketAddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("could not parse address: {}", addr))?;
    Ok(addrs)
}

// For each Spin app variable, checks if a container environment variable with
// the same name exists and duplicates it in the environment with the
// application variable prefix
pub(crate) fn configure_application_variables_from_environment_variables(
    resolved: &LockedApp,
) -> Result<()> {
    resolved
        .variables
        .keys()
        .map(|k| k.as_ref())
        .map(str::to_ascii_uppercase)
        .for_each(|variable| {
            env::var(&variable)
                .map(|val| {
                    let prefixed = format!(
                        "{}_{}",
                        constants::SPIN_APPLICATION_VARIABLE_PREFIX,
                        variable
                    );
                    env::set_var(prefixed, val);
                })
                .ok();
        });
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_configure_application_variables_from_environment_variables() {
        temp_env::with_vars(
            [
                ("SPIN_VARIABLE_DO_NOT_RESET", Some("val1")),
                ("SHOULD_BE_PREFIXED", Some("val2")),
                ("ignored_if_not_uppercased_env", Some("val3")),
            ],
            || {
                let app_json = r#"
                {
                    "spin_lock_version": 1,
                    "entrypoint": "test",
                    "components": [],
                    "variables": {"should_be_prefixed": { "required": "true"},  "do_not_reset" : { "required": "true"}, "not_set_as_container_env": { "required": "true"}, "ignored_if_not_uppercased_env": { "required": "true"}},
                    "triggers": []
                }"#;
                let locked_app = LockedApp::from_json(app_json.as_bytes()).unwrap();

                configure_application_variables_from_environment_variables(&locked_app).unwrap();
                assert_eq!(env::var("SPIN_VARIABLE_DO_NOT_RESET").unwrap(), "val1");
                assert_eq!(
                    env::var("SPIN_VARIABLE_SHOULD_BE_PREFIXED").unwrap(),
                    "val2"
                );
                assert!(env::var("SPIN_VARIABLE_NOT_SET_AS_CONTAINER_ENV").is_err());
                assert!(env::var("SPIN_VARIABLE_IGNORED_IF_NOT_UPPERCASED_ENV").is_err());
                // Original env vars are still retained but not set in variable provider
                assert!(env::var("SHOULD_BE_PREFIXED").is_ok());
            },
        );
    }

    #[test]
    fn can_parse_spin_address() {
        let parsed = parse_addr(constants::SPIN_ADDR_DEFAULT).unwrap();
        assert_eq!(parsed.clone().port(), 80);
        assert_eq!(parsed.ip().to_string(), "0.0.0.0");
    }

    #[test]
    fn is_wasm_content_test() {
        let wasm_content = WasmLayer {
            layer: vec![],
            config: oci_spec::image::Descriptor::new(
                MediaType::Other(constants::OCI_LAYER_MEDIA_TYPE_WASM.to_string()),
                1024,
                "sha256:1234",
            ),
        };
        // Should be ignored
        let data_content = WasmLayer {
            layer: vec![],
            config: oci_spec::image::Descriptor::new(
                MediaType::Other(spin_oci::client::DATA_MEDIATYPE.to_string()),
                1024,
                "sha256:1234",
            ),
        };
        assert!(is_wasm_content(&wasm_content).is_some());
        assert!(is_wasm_content(&data_content).is_none());
    }
}
