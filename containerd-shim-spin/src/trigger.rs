use std::collections::HashSet;

use anyhow::anyhow;
use spin_app::locked::LockedApp;
use spin_trigger::Trigger;
// use spin_trigger::{loader, RuntimeConfig, TriggerExecutor, TriggerExecutorBuilder};
use spin_trigger_http::HttpTrigger;
// use spin_trigger_redis::RedisTrigger;
// use trigger_command::CommandTrigger;
// use trigger_mqtt::MqttTrigger;
// use trigger_sqs::SqsTrigger;

pub(crate) const HTTP_TRIGGER_TYPE: &str = <HttpTrigger as Trigger<
    <spin_runtime_factors::FactorsBuilder as spin_trigger::cli::RuntimeFactorsBuilder>::Factors,
>>::TYPE;

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
///
/// Note: this function returns a `HashSet` of supported trigger types. Duplicates are removed.
pub(crate) fn get_supported_triggers(locked_app: &LockedApp) -> anyhow::Result<HashSet<String>> {
    let supported_triggers: HashSet<&str> = HashSet::from([
        // RedisTrigger::TRIGGER_TYPE,
        HTTP_TRIGGER_TYPE,
        // SqsTrigger::TRIGGER_TYPE,
        // MqttTrigger::TRIGGER_TYPE,
        // CommandTrigger::TRIGGER_TYPE,
    ]);

    locked_app.triggers.iter()
        .map(|trigger| {
            let trigger_type = &trigger.trigger_type;
            if !supported_triggers.contains(trigger_type.as_str()) {
                Err(anyhow!(
                    "Only Http, Redis, MQTT, SQS, and Command triggers are currently supported. Found unsupported trigger: {:?}",
                    trigger_type
                ))
            } else {
                Ok(trigger_type.clone())
            }
        })
        .collect::<anyhow::Result<HashSet<_>>>()
}
