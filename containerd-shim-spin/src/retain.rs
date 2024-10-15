//! This module contains the logic for modifying a locked app to only contain a subset of its components

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use spin_app::locked::LockedApp;
use spin_factor_outbound_networking::{allowed_outbound_hosts, parse_service_chaining_target};

/// Scrubs the locked app to only contain the given list of components
/// Introspects the LockedApp to find and selectively retain the triggers that correspond to those components
pub fn retain_components(locked_app: &mut LockedApp, retained_components: &[String]) -> Result<()> {
    // Create a temporary app to access parsed component and trigger information
    let tmp_app = spin_app::App::new("tmp", locked_app.clone());
    validate_retained_components_exist(&tmp_app, retained_components)?;
    validate_retained_components_service_chaining(&tmp_app, retained_components)?;
    let (component_ids, trigger_ids): (HashSet<String>, HashSet<String>) = tmp_app
        .triggers()
        .filter_map(|t| match t.component() {
            Ok(comp) if retained_components.contains(&comp.id().to_string()) => {
                Some((comp.id().to_owned(), t.id().to_owned()))
            }
            _ => None,
        })
        .collect();
    locked_app
        .components
        .retain(|c| component_ids.contains(&c.id));
    locked_app.triggers.retain(|t| trigger_ids.contains(&t.id));
    Ok(())
}

// Validates that all service chaining of an app will be satisfied by the
// retained components.
//
// This does a best effort look up of components that are
// allowed to be accessed through service chaining and will error early if a
// component is configured to to chain to another component that is not
// retained. All wildcard service chaining is disallowed and all templated URLs
// are ignored.
fn validate_retained_components_service_chaining(
    app: &spin_app::App,
    retained_components: &[String],
) -> Result<()> {
    app
        .triggers().try_for_each(|t| {
            let Ok(component) = t.component() else  { return Ok(()) };
            if retained_components.contains(&component.id().to_string()) {
            let allowed_hosts = allowed_outbound_hosts(&component).context("failed to get allowed hosts")?;
            for host in allowed_hosts {
                // Templated URLs are not yet resolved at this point, so ignore unresolvable URIs
                if let Ok(uri) = host.parse::<http::Uri>() {
                    if let Some(chaining_target) = parse_service_chaining_target(&uri) {
                        if !retained_components.contains(&chaining_target) {
                            if chaining_target == "*" {
                                bail!("Component selected with '--component {}' cannot use wildcard service chaining: allowed_outbound_hosts = [\"http://*.spin.internal\"]", component.id());
                            }
                            bail!(
                                "Component selected with '--component {}' cannot use service chaining to unselected component: allowed_outbound_hosts = [\"http://{}.spin.internal\"]",
                                component.id(), chaining_target
                            );
                        }
                    }
                }
            }
        }
        anyhow::Ok(())
    })?;

    Ok(())
}

// Validates that all components specified to be retained actually exist in the app
fn validate_retained_components_exist(
    app: &spin_app::App,
    retained_components: &[String],
) -> Result<()> {
    let app_components = app
        .components()
        .map(|c| c.id().to_string())
        .collect::<HashSet<_>>();
    for c in retained_components {
        if !app_components.contains(c) {
            bail!("Specified component \"{c}\" not found in application");
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    pub async fn build_locked_app(
        manifest: &toml::map::Map<String, toml::Value>,
    ) -> anyhow::Result<LockedApp> {
        let toml_str = toml::to_string(manifest).context("failed serializing manifest")?;
        let dir = tempfile::tempdir().context("failed creating tempdir")?;
        let path = dir.path().join("spin.toml");
        std::fs::write(&path, toml_str).context("failed writing manifest")?;
        spin_loader::from_file(&path, spin_loader::FilesMountStrategy::Direct, None).await
    }

    #[tokio::test]
    async fn test_retain_components_filtering_for_only_component_works() {
        let manifest = toml::toml! {
            spin_manifest_version = 2

            [application]
            name = "test-app"

            [[trigger.test-trigger]]
            component = "empty"

            [component.empty]
            source = "does-not-exist.wasm"
        };
        let mut locked_app = build_locked_app(&manifest).await.unwrap();
        retain_components(&mut locked_app, &["empty".to_string()]).unwrap();
        let components = locked_app
            .components
            .iter()
            .map(|c| c.id.to_string())
            .collect::<HashSet<_>>();
        assert!(components.contains("empty"));
        assert!(components.len() == 1);
    }

    #[tokio::test]
    async fn test_retain_components_filtering_for_non_existent_component_fails() {
        let manifest = toml::toml! {
            spin_manifest_version = 2

            [application]
            name = "test-app"

            [[trigger.test-trigger]]
            component = "empty"

            [component.empty]
            source = "does-not-exist.wasm"
        };
        let mut locked_app = build_locked_app(&manifest).await.unwrap();
        let Err(e) = retain_components(&mut locked_app, &["dne".to_string()]) else {
            panic!("Expected component not found error");
        };
        assert_eq!(
            e.to_string(),
            "Specified component \"dne\" not found in application"
        );
        assert!(retain_components(&mut locked_app, &["dne".to_string()]).is_err());
    }

    #[tokio::test]
    async fn test_retain_components_app_with_service_chaining_fails() {
        let manifest = toml::toml! {
            spin_manifest_version = 2

            [application]
            name = "test-app"

            [[trigger.test-trigger]]
            component = "empty"

            [component.empty]
            source = "does-not-exist.wasm"
            allowed_outbound_hosts = ["http://another.spin.internal"]

            [[trigger.another-trigger]]
            component = "another"

            [component.another]
            source = "does-not-exist.wasm"

            [[trigger.third-trigger]]
            component = "third"

            [component.third]
            source = "does-not-exist.wasm"
            allowed_outbound_hosts = ["http://*.spin.internal"]
        };
        let mut locked_app = build_locked_app(&manifest)
            .await
            .expect("could not build locked app");
        let Err(e) = retain_components(&mut locked_app, &["empty".to_string()]) else {
            panic!("Expected service chaining to non-retained component error");
        };
        assert_eq!(
            e.to_string(),
            "Component selected with '--component empty' cannot use service chaining to unselected component: allowed_outbound_hosts = [\"http://another.spin.internal\"]"
        );
        let Err(e) = retain_components(
            &mut locked_app,
            &["third".to_string(), "another".to_string()],
        ) else {
            panic!("Expected wildcard service chaining error");
        };
        assert_eq!(
            e.to_string(),
            "Component selected with '--component third' cannot use wildcard service chaining: allowed_outbound_hosts = [\"http://*.spin.internal\"]"
        );
        assert!(retain_components(&mut locked_app, &["another".to_string()]).is_ok());
    }

    #[tokio::test]
    async fn test_retain_components_app_with_templated_host_passes() {
        let manifest = toml::toml! {
            spin_manifest_version = 2

            [application]
            name = "test-app"

            [variables]
            host = { default = "test" }

            [[trigger.test-trigger]]
            component = "empty"

            [component.empty]
            source = "does-not-exist.wasm"

            [[trigger.another-trigger]]
            component = "another"

            [component.another]
            source = "does-not-exist.wasm"

            [[trigger.third-trigger]]
            component = "third"

            [component.third]
            source = "does-not-exist.wasm"
            allowed_outbound_hosts = ["http://{{ host }}.spin.internal"]
        };
        let mut locked_app = build_locked_app(&manifest)
            .await
            .expect("could not build locked app");
        assert!(
            retain_components(&mut locked_app, &["empty".to_string(), "third".to_string()]).is_ok()
        );
    }
}
