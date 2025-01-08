use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use anyhow::Context as _;
use async_trait::async_trait;
use containerd_shim_wasm::{container::PrecompiledLayer, sandbox::WasmLayer};
use spin_app::locked::LockedApp;
use spin_common::sha256;

use oci_spec::image::Digest;

use crate::constants::OCI_LAYER_MEDIA_TYPE_WASM;

/// Compose each layer with its dependencies and precompile.
pub async fn compose_and_precompile(
    precompile_engine: &wasmtime::Engine,
    layers: &[WasmLayer],
) -> anyhow::Result<Vec<PrecompiledLayer>> {
    let (locked_parent_digest, mut locked_app) = locked_app_from_layers(layers)?;

    let mut precompiled_layers = vec![];

    for component in locked_app.components.iter_mut() {
        let loader = ComponentSourceLoader::new(layers);

        let composed = spin_compose::compose(&loader, &component)
            .await
            .with_context(|| {
                format!(
                    "failed to resolve dependencies for component {:?}",
                    component.id
                )
            })?;

        let precompiled = precompile_engine.precompile_component(&composed)?;
        let precompiled_digest = format!("sha256:{}", sha256::hex_digest_from_bytes(&precompiled));

        log::debug!("Replacing component digest with precompiled digest: {precompiled_digest}");
        component
            .source
            .content
            .digest
            .replace(precompiled_digest.clone());

        let parents = loader.parents.lock().unwrap().clone();

        // Clear the dependencies to signal precompilation has taken place.
        component.dependencies.clear();

        precompiled_layers.push(PrecompiledLayer {
            media_type: OCI_LAYER_MEDIA_TYPE_WASM.to_string(),
            bytes: precompiled,
            parents,
        });
    }

    precompiled_layers.push(PrecompiledLayer {
        media_type: spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE.to_string(),
        bytes: locked_app.to_json()?,
        parents: {
            let mut parents = BTreeSet::new();
            parents.insert(locked_parent_digest.to_string());
            parents
        },
    });

    Ok(precompiled_layers)
}

// Returns the digest of the layer containing the LockedApp and the deserialized LockedApp.
fn locked_app_from_layers(layers: &[WasmLayer]) -> anyhow::Result<(Digest, LockedApp)> {
    let spin_layer = layers
        .iter()
        .find(|layer| match layer.config.media_type() {
            oci_spec::image::MediaType::Other(name)
                if name == spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE =>
            {
                true
            }
            _ => false,
        })
        .context("No Spin layer found")?;

    let locked_app = LockedApp::from_json(&spin_layer.layer)?;
    let digest = spin_layer.config.digest().clone();
    Ok((digest, locked_app))
}

struct ComponentSourceLoader<'a> {
    parents: Arc<Mutex<BTreeSet<String>>>,
    layers: &'a [WasmLayer],
}

impl<'a> ComponentSourceLoader<'a> {
    fn new(layers: &'a [WasmLayer]) -> Self {
        Self {
            parents: Arc::new(Mutex::new(BTreeSet::new())),
            layers,
        }
    }
}

#[async_trait]
impl<'a> spin_compose::ComponentSourceLoader for ComponentSourceLoader<'a> {
    async fn load_component_source(
        &self,
        source: &spin_app::locked::LockedComponentSource,
    ) -> anyhow::Result<Vec<u8>> {
        let digest = source
            .content
            .digest
            .as_ref()
            .context("LockedComponentSource missing digest field")?;

        let layer = self
            .layers
            .iter()
            .find(|layer| layer.config.digest().as_ref() == digest)
            .context("LockedComponentSource digest not found in layers")?;

        let component = spin_componentize::componentize_if_necessary(&layer.layer)?;

        let parent_digest = layer
            .config
            .digest()
            .to_string();

        // Insert the parent digest into the parents set
        self.parents.lock().unwrap().insert(parent_digest);

        Ok(component.into())
    }
}
