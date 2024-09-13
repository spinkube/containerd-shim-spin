/// SPIN_ADDR_DEFAULT is the default address and port that the Spin HTTP trigger
/// listens on.
pub(crate) const SPIN_ADDR_DEFAULT: &str = "0.0.0.0:80";
/// SPIN_HTTP_LISTEN_ADDR_ENV is the environment variable that can be used to
/// override the default address and port that the Spin HTTP trigger listens on.
pub(crate) const SPIN_HTTP_LISTEN_ADDR_ENV: &str = "SPIN_HTTP_LISTEN_ADDR";
/// RUNTIME_CONFIG_PATH specifies the expected location and name of the runtime
/// config for a Spin application. The runtime config should be loaded into the
/// root `/` of the container.
pub(crate) const RUNTIME_CONFIG_PATH: &str = "/runtime-config.toml";
/// Describes an OCI layer with Wasm content
pub(crate) const OCI_LAYER_MEDIA_TYPE_WASM: &str = "application/vnd.wasm.content.layer.v1+wasm";
/// Expected location of the Spin manifest when loading from a file rather than
/// an OCI image
pub(crate) const SPIN_MANIFEST_FILE_PATH: &str = "/spin.toml";
/// Known prefix for the Spin application variables environment variable
/// provider: https://github.com/fermyon/spin/blob/436ad589237c02f7aa4693e984132808fd80b863/crates/variables/src/provider/env.rs#L9
pub(crate) const SPIN_APPLICATION_VARIABLE_PREFIX: &str = "SPIN_VARIABLE";
/// Working directory for Spin applications
pub(crate) const SPIN_TRIGGER_WORKING_DIR: &str = "/";
