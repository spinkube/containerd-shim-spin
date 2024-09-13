// use anyhow::Result;
// use spin_trigger::{RuntimeConfig, TriggerHooks};

// pub(crate) struct StdioHook;

// impl TriggerHooks for StdioHook {
//     fn app_loaded(
//         &mut self,
//         _app: &spin_app::App,
//         _runtime_config: &RuntimeConfig,
//         _resolver: &std::sync::Arc<spin_expressions::PreparedResolver>,
//     ) -> Result<()> {
//         Ok(())
//     }

//     fn component_store_builder(
//         &self,
//         _component: &spin_app::AppComponent,
//         builder: &mut spin_core::StoreBuilder,
//     ) -> Result<()> {
//         builder.inherit_stdout();
//         builder.inherit_stderr();
//         Ok(())
//     }
// }

// TODO: make sure we're inheriting stdout/stderr from the right place
