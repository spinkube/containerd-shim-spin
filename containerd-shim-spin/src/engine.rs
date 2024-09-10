use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    env,
    fs::File,
    hash::{Hash, Hasher},
    io::Write,
    net::{SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Result};
use containerd_shim_wasm::{
    container::{Engine, RuntimeContext, Stdio},
    sandbox::WasmLayer,
    version,
};
use futures::future;
use log::info;
use oci_spec::image::MediaType;
use spin_app::locked::LockedApp;
use spin_loader::{cache::Cache, FilesMountStrategy};
use spin_manifest::schema::v2::AppManifest;
use spin_trigger::{loader, RuntimeConfig, TriggerExecutor, TriggerExecutorBuilder, TriggerHooks};
use spin_trigger_http::HttpTrigger;
use spin_trigger_redis::RedisTrigger;
use tokio::runtime::Runtime;
use trigger_command::CommandTrigger;
use trigger_mqtt::MqttTrigger;
use trigger_sqs::SqsTrigger;
use url::Url;

/// SPIN_ADDR_DEFAULT is the default address and port that the Spin HTTP trigger
/// listens on.
const SPIN_ADDR_DEFAULT: &str = "0.0.0.0:80";
/// SPIN_HTTP_LISTEN_ADDR_ENV is the environment variable that can be used to
/// override the default address and port that the Spin HTTP trigger listens on.
const SPIN_HTTP_LISTEN_ADDR_ENV: &str = "SPIN_HTTP_LISTEN_ADDR";
/// RUNTIME_CONFIG_PATH specifies the expected location and name of the runtime
/// config for a Spin application. The runtime config should be loaded into the
/// root `/` of the container.
const RUNTIME_CONFIG_PATH: &str = "/runtime-config.toml";
/// Describes an OCI layer with Wasm content
const OCI_LAYER_MEDIA_TYPE_WASM: &str = "application/vnd.wasm.content.layer.v1+wasm";
/// Expected location of the Spin manifest when loading from a file rather than
/// an OCI image
const SPIN_MANIFEST_FILE_PATH: &str = "/spin.toml";
/// Known prefix for the Spin application variables environment variable
/// provider: https://github.com/fermyon/spin/blob/436ad589237c02f7aa4693e984132808fd80b863/crates/variables/src/provider/env.rs#L9
const SPIN_APPLICATION_VARIABLE_PREFIX: &str = "SPIN_VARIABLE";

#[derive(Clone)]
pub struct SpinEngine {
    pub(crate) wasmtime_engine: wasmtime::Engine,
}

impl Default for SpinEngine {
    fn default() -> Self {
        // the host expects epoch interruption to be enabled, so this has to be
        // turned on for the components we compile.
        let mut config = wasmtime::Config::default();
        config.epoch_interruption(true);
        Self {
            wasmtime_engine: wasmtime::Engine::new(&config).unwrap(),
        }
    }
}

struct StdioTriggerHook;
impl TriggerHooks for StdioTriggerHook {
    fn app_loaded(
        &mut self,
        _app: &spin_app::App,
        _runtime_config: &RuntimeConfig,
        _resolver: &std::sync::Arc<spin_expressions::PreparedResolver>,
    ) -> Result<()> {
        Ok(())
    }

    fn component_store_builder(
        &self,
        _component: &spin_app::AppComponent,
        builder: &mut spin_core::StoreBuilder,
    ) -> Result<()> {
        builder.inherit_stdout();
        builder.inherit_stderr();
        Ok(())
    }
}

#[derive(Clone)]
enum AppSource {
    File(PathBuf),
    Oci,
}

impl std::fmt::Debug for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppSource::File(path) => write!(f, "File({})", path.display()),
            AppSource::Oci => write!(f, "Oci"),
        }
    }
}

impl SpinEngine {
    async fn app_source(&self, ctx: &impl RuntimeContext, cache: &Cache) -> Result<AppSource> {
        match ctx.entrypoint().source {
            containerd_shim_wasm::container::Source::File(_) => {
                Ok(AppSource::File(SPIN_MANIFEST_FILE_PATH.into()))
            }
            containerd_shim_wasm::container::Source::Oci(layers) => {
                info!(" >>> configuring spin oci application {}", layers.len());

                for layer in layers {
                    log::debug!("<<< layer config: {:?}", layer.config);
                }

                for artifact in layers {
                    match artifact.config.media_type() {
                        MediaType::Other(name)
                            if name == spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE =>
                        {
                            let path = PathBuf::from("/spin.json");
                            log::info!("writing spin oci config to {:?}", path);
                            File::create(&path)
                                .context("failed to create spin.json")?
                                .write_all(&artifact.layer)
                                .context("failed to write spin.json")?;
                        }
                        MediaType::Other(name) if name == OCI_LAYER_MEDIA_TYPE_WASM => {
                            log::info!(
                                "<<< writing wasm artifact with length {:?} config to cache, near {:?}",
                                artifact.layer.len(), cache.manifests_dir()
                            );
                            cache
                                .write_wasm(&artifact.layer, &artifact.config.digest())
                                .await?;
                        }
                        MediaType::Other(name) if name == spin_oci::client::DATA_MEDIATYPE => {
                            log::debug!(
                                "<<< writing data layer to cache, near {:?}",
                                cache.manifests_dir()
                            );
                            cache
                                .write_data(&artifact.layer, &artifact.config.digest())
                                .await?;
                        }
                        MediaType::Other(name) if name == spin_oci::client::ARCHIVE_MEDIATYPE => {
                            log::debug!(
                                "<<< writing archive layer and unpacking contents to cache, near {:?}",
                                cache.manifests_dir()
                            );
                            self.handle_archive_layer(
                                cache,
                                &artifact.layer,
                                &artifact.config.digest(),
                            )
                            .await
                            .context("unable to unpack archive layer")?;
                        }
                        _ => {
                            log::debug!(
                                "<<< unknown media type {:?}",
                                artifact.config.media_type()
                            );
                        }
                    }
                }
                Ok(AppSource::Oci)
            }
        }
    }

    async fn resolve_app_source(
        &self,
        app_source: AppSource,
        cache: &Cache,
    ) -> Result<ResolvedAppSource> {
        let resolve_app_source = match app_source {
            AppSource::File(source) => ResolvedAppSource::File {
                manifest_path: source.clone(),
                manifest: spin_manifest::manifest_from_file(source.clone())?,
            },
            AppSource::Oci => {
                let working_dir = PathBuf::from("/");
                let loader = spin_oci::OciLoader::new(working_dir);

                // TODO: what is the best way to get this info? It isn't used only saved in the locked file
                let reference = "docker.io/library/wasmtest_spin:latest";

                let locked_app = loader
                    .load_from_cache(PathBuf::from("/spin.json"), reference, cache)
                    .await?;
                ResolvedAppSource::OciRegistry { locked_app }
            }
        };
        Ok(resolve_app_source)
    }

    async fn wasm_exec_async(&self, ctx: &impl RuntimeContext) -> Result<()> {
        // create a cache directory at /.cache
        // this is needed for the spin LocalLoader to work
        // TODO: spin should provide a more flexible `loader::from_file` that
        // does not assume the existence of a cache directory
        let cache_dir = PathBuf::from("/.cache");
        let cache = Cache::new(Some(cache_dir.clone()))
            .await
            .context("failed to create cache")?;
        env::set_var("XDG_CACHE_HOME", &cache_dir);
        let app_source = self.app_source(ctx, &cache).await?;
        let resolved_app_source = self.resolve_app_source(app_source.clone(), &cache).await?;
        configure_application_variables_from_environment_variables(&resolved_app_source)?;
        let trigger_cmds = trigger_command_for_resolved_app_source(&resolved_app_source)
            .with_context(|| format!("Couldn't find trigger executor for {app_source:?}"))?;
        let locked_app = self.load_resolved_app_source(resolved_app_source).await?;

        let _telemetry_guard = spin_telemetry::init(version!().to_string())?;

        self.run_trigger(
            ctx,
            trigger_cmds.iter().map(|s| s.as_ref()).collect(),
            locked_app,
            app_source,
        )
        .await
    }

    async fn run_trigger(
        &self,
        ctx: &impl RuntimeContext,
        trigger_types: Vec<&str>,
        app: LockedApp,
        app_source: AppSource,
    ) -> Result<()> {
        let working_dir = PathBuf::from("/");
        let mut futures_list = Vec::with_capacity(trigger_types.len());
        for trigger_type in trigger_types.iter() {
            let f = match trigger_type.to_owned() {
                HttpTrigger::TRIGGER_TYPE => {
                    let http_trigger: HttpTrigger = self
                        .build_spin_trigger(working_dir.clone(), app.clone(), app_source.clone())
                        .await
                        .context("failed to build spin trigger")?;

                    info!(" >>> running spin http trigger");
                    let address_str = env::var(SPIN_HTTP_LISTEN_ADDR_ENV)
                        .unwrap_or_else(|_| SPIN_ADDR_DEFAULT.to_string());
                    let address = parse_addr(&address_str)?;
                    http_trigger.run(spin_trigger_http::CliArgs {
                        address,
                        tls_cert: None,
                        tls_key: None,
                    })
                }
                RedisTrigger::TRIGGER_TYPE => {
                    let redis_trigger: RedisTrigger = self
                        .build_spin_trigger(working_dir.clone(), app.clone(), app_source.clone())
                        .await
                        .context("failed to build spin trigger")?;

                    info!(" >>> running spin redis trigger");
                    redis_trigger.run(spin_trigger::cli::NoArgs)
                }
                SqsTrigger::TRIGGER_TYPE => {
                    let sqs_trigger: SqsTrigger = self
                        .build_spin_trigger(working_dir.clone(), app.clone(), app_source.clone())
                        .await
                        .context("failed to build spin trigger")?;

                    info!(" >>> running spin trigger");
                    sqs_trigger.run(spin_trigger::cli::NoArgs)
                }
                CommandTrigger::TRIGGER_TYPE => {
                    let command_trigger: CommandTrigger = self
                        .build_spin_trigger(working_dir.clone(), app.clone(), app_source.clone())
                        .await
                        .context("failed to build spin trigger")?;

                    info!(" >>> running spin trigger");
                    command_trigger.run(trigger_command::CliArgs {
                        guest_args: ctx.args().to_vec(),
                    })
                }
                MqttTrigger::TRIGGER_TYPE => {
                    let mqtt_trigger: MqttTrigger = self
                        .build_spin_trigger(working_dir.clone(), app.clone(), app_source.clone())
                        .await
                        .context("failed to build spin trigger")?;

                    info!(" >>> running spin trigger");
                    mqtt_trigger.run(trigger_mqtt::CliArgs { test: false })
                }
                _ => {
                    todo!(
                        "Only Http, Redis, MQTT, SQS and Command triggers are currently supported."
                    )
                }
            };

            futures_list.push(f)
        }

        info!(" >>> notifying main thread we are about to start");

        // exit as soon as any of the trigger completes/exits
        let (result, index, rest) = future::select_all(futures_list).await;
        info!(
            " >>> trigger type '{trigger_type}' exited",
            trigger_type = trigger_types[index]
        );

        drop(rest);

        result
    }

    async fn load_resolved_app_source(
        &self,
        resolved: ResolvedAppSource,
    ) -> anyhow::Result<LockedApp> {
        match resolved {
            ResolvedAppSource::File { manifest_path, .. } => {
                // TODO: This should be configurable, see https://github.com/deislabs/containerd-wasm-shims/issues/166
                // TODO: ^^ Move aforementioned issue to this repo
                let files_mount_strategy = FilesMountStrategy::Direct;
                spin_loader::from_file(&manifest_path, files_mount_strategy, None).await
            }
            ResolvedAppSource::OciRegistry { locked_app } => Ok(locked_app),
        }
    }

    async fn write_locked_app(&self, locked_app: &LockedApp, working_dir: &Path) -> Result<String> {
        let locked_path = working_dir.join("spin.lock");
        let locked_app_contents =
            serde_json::to_vec_pretty(&locked_app).context("failed to serialize locked app")?;
        tokio::fs::write(&locked_path, locked_app_contents)
            .await
            .with_context(|| format!("failed to write {:?}", locked_path))?;
        let locked_url = Url::from_file_path(&locked_path)
            .map_err(|_| anyhow!("cannot convert to file URL: {locked_path:?}"))?
            .to_string();

        Ok(locked_url)
    }

    async fn build_spin_trigger<T: spin_trigger::TriggerExecutor>(
        &self,
        working_dir: PathBuf,
        app: LockedApp,
        app_source: AppSource,
    ) -> Result<T>
    where
        for<'de> <T as TriggerExecutor>::TriggerConfig: serde::de::Deserialize<'de>,
    {
        let locked_url = self.write_locked_app(&app, &working_dir).await?;

        // Build trigger config
        let mut loader = loader::TriggerLoader::new(working_dir.clone(), true);
        match app_source {
            AppSource::Oci => unsafe {
                // Configure the loader to support loading AOT compiled components..
                // Since all components were compiled by the shim (during `precompile`),
                // this operation can be considered safe.
                loader.enable_loading_aot_compiled_components();
            },
            // Currently, it is only possible to precompile applications distributed using
            // `spin registry push`
            AppSource::File(_) => {}
        };
        let mut runtime_config = RuntimeConfig::new(PathBuf::from("/").into());
        // Load in runtime config if one exists at expected location
        if Path::new(RUNTIME_CONFIG_PATH).exists() {
            runtime_config.merge_config_file(RUNTIME_CONFIG_PATH)?;
        }
        let mut builder = TriggerExecutorBuilder::new(loader);
        builder
            .hooks(StdioTriggerHook {})
            .config_mut()
            .wasmtime_config()
            .cranelift_opt_level(spin_core::wasmtime::OptLevel::Speed);
        let init_data = Default::default();
        let executor = builder.build(locked_url, runtime_config, init_data).await?;
        Ok(executor)
    }

    // Returns Some(WasmLayer) if the layer contains wasm, otherwise None
    fn is_wasm_content(layer: &WasmLayer) -> Option<WasmLayer> {
        if let MediaType::Other(name) = layer.config.media_type() {
            if name == OCI_LAYER_MEDIA_TYPE_WASM {
                return Some(layer.clone());
            }
        }
        None
    }

    async fn handle_archive_layer(
        &self,
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
}

impl Engine for SpinEngine {
    fn name() -> &'static str {
        "spin"
    }

    fn run_wasi(&self, ctx: &impl RuntimeContext, stdio: Stdio) -> Result<i32> {
        stdio.redirect()?;
        info!("setting up wasi");
        let rt = Runtime::new().context("failed to create runtime")?;

        let (abortable, abort_handle) = futures::future::abortable(self.wasm_exec_async(ctx));
        ctrlc::set_handler(move || abort_handle.abort())?;

        match rt.block_on(abortable) {
            Ok(Ok(())) => {
                info!("run_wasi shut down: exiting");
                Ok(0)
            }
            Ok(Err(err)) => {
                log::error!("run_wasi ERROR >>>  failed: {:?}", err);
                Err(err)
            }
            Err(aborted) => {
                info!("Received signal to abort: {:?}", aborted);
                Ok(0)
            }
        }
    }

    fn can_handle(&self, _ctx: &impl RuntimeContext) -> Result<()> {
        Ok(())
    }

    fn supported_layers_types() -> &'static [&'static str] {
        &[
            OCI_LAYER_MEDIA_TYPE_WASM,
            spin_oci::client::ARCHIVE_MEDIATYPE,
            spin_oci::client::DATA_MEDIATYPE,
            spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE,
        ]
    }

    fn precompile(&self, layers: &[WasmLayer]) -> Result<Vec<Option<Vec<u8>>>> {
        // Runwasi expects layers to be returned in the same order, so wrap each layer in an option, setting non Wasm layers to None
        let precompiled_layers = layers
            .iter()
            .map(|layer| match SpinEngine::is_wasm_content(layer) {
                Some(wasm_layer) => {
                    log::info!(
                        "Precompile called for wasm layer {:?}",
                        wasm_layer.config.digest()
                    );
                    if self
                        .wasmtime_engine
                        .detect_precompiled(&wasm_layer.layer)
                        .is_some()
                    {
                        log::info!("Layer already precompiled {:?}", wasm_layer.config.digest());
                        Ok(Some(wasm_layer.layer))
                    } else {
                        let component =
                            spin_componentize::componentize_if_necessary(&wasm_layer.layer)?;
                        let precompiled = self.wasmtime_engine.precompile_component(&component)?;
                        Ok(Some(precompiled))
                    }
                }
                None => Ok(None),
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(precompiled_layers)
    }

    fn can_precompile(&self) -> Option<String> {
        let mut hasher = DefaultHasher::new();
        self.wasmtime_engine
            .precompile_compatibility_hash()
            .hash(&mut hasher);
        Some(hasher.finish().to_string())
    }
}

fn parse_addr(addr: &str) -> Result<SocketAddr> {
    let addrs: SocketAddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("could not parse address: {}", addr))?;
    Ok(addrs)
}

// TODO: we should use spin's ResolvedAppSource
pub enum ResolvedAppSource {
    File {
        manifest_path: PathBuf,
        manifest: AppManifest,
    },
    OciRegistry {
        locked_app: LockedApp,
    },
}

impl ResolvedAppSource {
    pub fn trigger_types(&self) -> anyhow::Result<Vec<&str>> {
        let types = match self {
            ResolvedAppSource::File { manifest, .. } => {
                manifest.triggers.keys().collect::<HashSet<_>>()
            }
            ResolvedAppSource::OciRegistry { locked_app } => locked_app
                .triggers
                .iter()
                .map(|t| &t.trigger_type)
                .collect::<HashSet<_>>(),
        };

        ensure!(!types.is_empty(), "no triggers in app");
        Ok(types.into_iter().map(|t| t.as_str()).collect())
    }

    pub fn variables(&self) -> Vec<&str> {
        match self {
            ResolvedAppSource::File { manifest, .. } => manifest
                .variables
                .keys()
                .map(|k| k.as_ref())
                .collect::<Vec<_>>(),
            ResolvedAppSource::OciRegistry { locked_app } => locked_app
                .variables
                .keys()
                .map(|k| k.as_ref())
                .collect::<Vec<_>>(),
        }
    }
}

fn trigger_command_for_resolved_app_source(resolved: &ResolvedAppSource) -> Result<Vec<String>> {
    let trigger_types = resolved.trigger_types()?;
    let mut types = Vec::with_capacity(trigger_types.len());
    for trigger_type in trigger_types.iter() {
        match trigger_type.to_owned() {
            RedisTrigger::TRIGGER_TYPE
            | HttpTrigger::TRIGGER_TYPE
            | SqsTrigger::TRIGGER_TYPE
            | MqttTrigger::TRIGGER_TYPE
            | CommandTrigger::TRIGGER_TYPE => types.push(trigger_type),
            _ => {
                todo!("Only Http, Redis, MQTT and SQS triggers are currently supported.")
            }
        }
    }

    Ok(trigger_types.iter().map(|x| x.to_string()).collect())
}

// For each Spin app variable, checks if a container environment variable with
// the same name exists and duplicates it in the environment with the
// application variable prefix
fn configure_application_variables_from_environment_variables(
    resolved: &ResolvedAppSource,
) -> Result<()> {
    resolved
        .variables()
        .into_iter()
        .map(str::to_ascii_uppercase)
        .for_each(|variable| {
            env::var(&variable)
                .map(|val| {
                    let prefixed = format!("{}_{}", SPIN_APPLICATION_VARIABLE_PREFIX, variable);
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
                let resolved = ResolvedAppSource::OciRegistry { locked_app };

                configure_application_variables_from_environment_variables(&resolved).unwrap();
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
        let parsed = parse_addr(SPIN_ADDR_DEFAULT).unwrap();
        assert_eq!(parsed.clone().port(), 80);
        assert_eq!(parsed.ip().to_string(), "0.0.0.0");
    }

    #[test]
    fn is_wasm_content() {
        let wasm_content = WasmLayer {
            layer: vec![],
            config: oci_spec::image::Descriptor::new(
                MediaType::Other(OCI_LAYER_MEDIA_TYPE_WASM.to_string()),
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
        assert!(SpinEngine::is_wasm_content(&wasm_content).is_some());
        assert!(SpinEngine::is_wasm_content(&data_content).is_none());
    }

    #[test]
    fn precompile() {
        let module = wat::parse_str("(module)").unwrap();
        let wasmtime_engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::new(&wasmtime_engine, "(component)")
            .unwrap()
            .serialize()
            .unwrap();
        let wasm_layers: Vec<WasmLayer> = vec![
            // Needs to be precompiled
            WasmLayer {
                layer: module.clone(),
                config: oci_spec::image::Descriptor::new(
                    MediaType::Other(OCI_LAYER_MEDIA_TYPE_WASM.to_string()),
                    1024,
                    "sha256:1234",
                ),
            },
            // Precompiled
            WasmLayer {
                layer: component.to_owned(),
                config: oci_spec::image::Descriptor::new(
                    MediaType::Other(OCI_LAYER_MEDIA_TYPE_WASM.to_string()),
                    1024,
                    "sha256:1234",
                ),
            },
            // Content that should be skipped
            WasmLayer {
                layer: vec![],
                config: oci_spec::image::Descriptor::new(
                    MediaType::Other(spin_oci::client::DATA_MEDIATYPE.to_string()),
                    1024,
                    "sha256:1234",
                ),
            },
        ];
        let spin_engine = SpinEngine::default();
        let precompiled = spin_engine
            .precompile(&wasm_layers)
            .expect("precompile failed");
        assert_eq!(precompiled.len(), 3);
        assert_ne!(precompiled[0].as_deref().expect("no first entry"), module);
        assert_eq!(
            precompiled[1].as_deref().expect("no second entry"),
            component
        );
        assert!(precompiled[2].is_none());
    }
}
