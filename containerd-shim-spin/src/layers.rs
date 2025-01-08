use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use anyhow::Context as _;
use async_trait::async_trait;
use containerd_shim_wasm::{container::PrecompiledLayer, sandbox::WasmLayer};
use spin_app::locked::LockedApp;
use spin_common::sha256;

use crate::constants::OCI_LAYER_MEDIA_TYPE_WASM;

/// Compose each layer with its dependencies and precompile.
pub async fn compose_and_precompile(
    precompile_engine: &wasmtime::Engine,
    layers: &[WasmLayer],
) -> anyhow::Result<Vec<PrecompiledLayer>> {
    let (locked_parent_idx, mut locked_app) = locked_app_from_layers(layers)?;

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
            parents.insert(locked_parent_idx);
            parents
        },
    });

    Ok(precompiled_layers)
}

// Returns the index of the layer containing the LockedApp and the LockedApp itself.
fn locked_app_from_layers(layers: &[WasmLayer]) -> anyhow::Result<(usize, LockedApp)> {
    let (parent_idx, spin_layer) =
        find_spin_app_layer(layers).ok_or_else(|| anyhow::anyhow!("No Spin layer found"))?;
    let locked_app = LockedApp::from_json(&spin_layer.layer)?;
    Ok((parent_idx, locked_app))
}

fn find_spin_app_layer(layers: &[WasmLayer]) -> Option<(usize, WasmLayer)> {
    for (i, layer) in layers.iter().enumerate() {
        match layer.config.media_type() {
            oci_spec::image::MediaType::Other(name)
                if name == spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE =>
            {
                return Some((i, layer.clone()));
            }
            _ => {}
        }
    }
    None
}

struct ComponentSourceLoader<'a> {
    parents: Arc<Mutex<BTreeSet<usize>>>,
    layers: &'a [WasmLayer],
}

impl<'a> ComponentSourceLoader<'a> {
    fn find_layer_by_digest(&self, digest: &str) -> Option<(usize, &WasmLayer)> {
        for (i, layer) in self.layers.iter().enumerate() {
            if layer.config.digest().as_ref() == digest {
                return Some((i, layer));
            }
        }
        None
    }

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

        let (idx, layer) = self
            .find_layer_by_digest(digest)
            .context("LockedComponentSource digest not found in layers")?;

        let component = spin_componentize::componentize_if_necessary(&layer.layer)?;

        // Insert the parent index into the parents set
        self.parents.lock().unwrap().insert(idx);

        Ok(component.into())
    }
}
