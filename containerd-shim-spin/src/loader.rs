use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};
use containerd_shim_wasm::sandbox::WasmLayer;
use spin_app::locked::{ContentPath, ContentRef, LockedApp, LockedComponent};
use url::Url;

pub struct ContainerdLoader {
    pub working_dir: PathBuf,
}

impl ContainerdLoader {
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        let working_dir = working_dir.into();
        Self { working_dir }
    }

    /// Load a Spin application given a list of layers that represent
    /// a Spin application distributed as an OCI artifact.
    pub async fn load_from_layers(&self, layers: &[WasmLayer]) -> Result<LockedApp> {
        // get the locked application file from the layer list
        let locked_layer = layers
            .iter()
            .find(|l| {
                l.config.media_type().to_string() == spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE
            })
            .context("cannot find locked application in layers for application")?;
        let mut locked_app = LockedApp::from_json(&locked_layer.layer)?;

        for component in &mut locked_app.components {
            self.resolve_component_content_refs(component, layers)
                .await
                .with_context(|| {
                    format!("failed to resolve content for component {:?}", component.id)
                })?;
        }

        Ok(locked_app)
    }

    async fn resolve_component_content_refs(
        &self,
        component: &mut LockedComponent,
        layers: &[WasmLayer],
    ) -> Result<()> {
        let wasm_digest = &component
            .source
            .content
            .digest
            .as_deref()
            .context("component must have source digest")?;
        let wasm = layers
            .iter()
            .find(|l| l.config.digest() == wasm_digest)
            .context("cannot find wasm source for component")?;
        let wasm_path = self.working_dir.join(wasm_digest);
        tokio::fs::write(&wasm_path, &wasm.layer).await?;
        component.source.content = Self::content_ref(wasm_path)?;

        if !component.files.is_empty() {
            let mount_dir = self.working_dir.join("assets").join(&component.id);
            for file in &mut component.files {
                ensure!(
                    Self::is_safe_to_join(&file.path),
                    "invalid file mount {file:?}"
                );
                let mount_path = mount_dir.join(&file.path);

                // Create parent directory
                let mount_parent = mount_path
                    .parent()
                    .with_context(|| format!("invalid mount path {mount_path:?}"))?;
                tokio::fs::create_dir_all(mount_parent)
                    .await
                    .with_context(|| {
                        format!("failed to create temporary mount path {mount_path:?}")
                    })?;

                if let Some(content_bytes) = file.content.inline.as_deref() {
                    // Write inline content to disk
                    tokio::fs::write(&mount_path, content_bytes)
                        .await
                        .with_context(|| {
                            format!("failed to write inline content to {mount_path:?}")
                        })?;
                } else {
                    // Copy content
                    let digest = Self::content_digest(&file.content)?;
                    let content_bytes = layers
                        .iter()
                        .find(|l| l.config.digest() == digest)
                        .context("cannot find static asset")?;
                    // Write inline content to disk
                    tokio::fs::write(&mount_path, &content_bytes.layer)
                        .await
                        .with_context(|| {
                            format!("failed to write inline content to {mount_path:?}")
                        })?;
                }
            }

            component.files = vec![ContentPath {
                content: Self::content_ref(mount_dir)?,
                path: "/".into(),
            }]
        }

        Ok(())
    }

    pub fn content_digest(content_ref: &ContentRef) -> Result<&str> {
        content_ref
            .digest
            .as_deref()
            .with_context(|| format!("content missing expected digest: {content_ref:?}"))
    }

    fn content_ref(path: impl AsRef<Path>) -> Result<ContentRef> {
        let path = std::fs::canonicalize(path)?;
        let url = Url::from_file_path(path).map_err(|_| anyhow!("couldn't build file URL"))?;
        Ok(ContentRef {
            source: Some(url.to_string()),
            ..Default::default()
        })
    }

    fn is_safe_to_join(path: impl AsRef<Path>) -> bool {
        // This could be loosened, but currently should always be true
        path.as_ref()
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
    }
}
