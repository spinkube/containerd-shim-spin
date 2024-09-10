use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use spin_app::locked::LockedApp;
use spin_trigger::{loader, RuntimeConfig, TriggerExecutor, TriggerExecutorBuilder};
use spin_trigger_http::HttpTrigger;
use spin_trigger_redis::RedisTrigger;
use trigger_command::CommandTrigger;
use trigger_mqtt::MqttTrigger;
use trigger_sqs::SqsTrigger;
use url::Url;

use crate::{constants, source::Source, stdio_hook::StdioHook};

pub(crate) async fn build_trigger<T>(app: LockedApp, app_source: Source) -> Result<T>
where
    T: spin_trigger::TriggerExecutor,
    T::TriggerConfig: serde::de::DeserializeOwned,
{
    let working_dir = PathBuf::from(constants::SPIN_TRIGGER_WORKING_DIR);
    let trigger: T = build_trigger_inner(working_dir, app, app_source)
        .await
        .context("failed to build spin trigger")?;
    Ok(trigger)
}

async fn build_trigger_inner<T: spin_trigger::TriggerExecutor>(
    working_dir: PathBuf,
    app: LockedApp,
    app_source: Source,
) -> Result<T>
where
    for<'de> <T as TriggerExecutor>::TriggerConfig: serde::de::Deserialize<'de>,
{
    let locked_url = write_locked_app(&app, &working_dir).await?;

    // Build trigger config
    let mut loader = loader::TriggerLoader::new(working_dir.clone(), true);
    match app_source {
        Source::Oci => unsafe {
            // Configure the loader to support loading AOT compiled components..
            // Since all components were compiled by the shim (during `precompile`),
            // this operation can be considered safe.
            loader.enable_loading_aot_compiled_components();
        },
        // Currently, it is only possible to precompile applications distributed using
        // `spin registry push`
        Source::File(_) => {}
    };
    let mut runtime_config = RuntimeConfig::new(PathBuf::from("/").into());
    // Load in runtime config if one exists at expected location
    if Path::new(constants::RUNTIME_CONFIG_PATH).exists() {
        runtime_config.merge_config_file(constants::RUNTIME_CONFIG_PATH)?;
    }
    let mut builder = TriggerExecutorBuilder::new(loader);
    builder
        .hooks(StdioHook {})
        .config_mut()
        .wasmtime_config()
        .cranelift_opt_level(spin_core::wasmtime::OptLevel::Speed);
    let init_data = Default::default();
    let executor = builder.build(locked_url, runtime_config, init_data).await?;
    Ok(executor)
}

async fn write_locked_app(locked_app: &LockedApp, working_dir: &Path) -> Result<String> {
    let locked_path: PathBuf = working_dir.join(constants::SPIN_LOCK_FILE_NAME);
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

/// get the supported trigger types from the `LockedApp`.
///
/// this function filters the trigger types to only return the ones that are currently supported.
/// If an unsupported trigger type is found, it returns an error indicating which trigger type is unsupported.
///
/// supported trigger types include:
/// - redis
/// - http
/// - sqs
/// - mqtt
/// - command
pub(crate) fn get_supported_triggers(locked_app: &LockedApp) -> anyhow::Result<Vec<String>> {
    let supported_triggers: HashSet<&str> = HashSet::from([
        RedisTrigger::TRIGGER_TYPE,
        HttpTrigger::TRIGGER_TYPE,
        SqsTrigger::TRIGGER_TYPE,
        MqttTrigger::TRIGGER_TYPE,
        CommandTrigger::TRIGGER_TYPE,
    ]);

    let mut types: Vec<String> = Vec::with_capacity(locked_app.triggers.len());

    for trigger in &locked_app.triggers {
        let trigger_type = &trigger.trigger_type;
        if !supported_triggers.contains(trigger_type.as_str()) {
            anyhow::bail!(
                "Only Http, Redis, MQTT, SQS, and Command triggers are currently supported. Found unsupported trigger: {:?}",
                trigger_type
            );
        }
        types.push(trigger_type.clone());
    }
    Ok(types)
}
