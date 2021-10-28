use rlua::{Function, Lua, Table};
use std::{fs, str::FromStr};

use crate::snapshot_middleware::SnapshotMiddleware;

pub struct PluginEnv {
    lua: Lua,
}

impl PluginEnv {
    pub fn new() -> Self {
        let lua = Lua::new();
        PluginEnv { lua }
    }

    pub fn init(&self) -> Result<(), rlua::Error> {
        self.lua.context(|lua_ctx| {
            let globals = lua_ctx.globals();

            let plugins_table = lua_ctx.create_table()?;
            globals.set("plugins", plugins_table)?;

            let run_plugins_fn = lua_ctx.create_function(|lua_ctx, id: String| {
                let plugins: Table = lua_ctx.globals().get("plugins")?;
                let id_ref: &str = &id;
                for plugin in plugins.sequence_values::<Table>() {
                    let load: Function = plugin?.get("load")?;
                    load.call(id_ref)?;
                }

                Ok(())
            })?;
            globals.set("runPlugins", run_plugins_fn)?;

            Ok::<(), rlua::Error>(())
        })
    }

    fn load_plugin_source(&self, plugin_source: &str) -> String {
        // TODO: Support downloading and caching plugins
        fs::read_to_string(plugin_source).unwrap()
    }

    pub fn load_plugin(
        &self,
        plugin_source: &str,
        plugin_options: String,
    ) -> Result<(), rlua::Error> {
        let plugin_lua = &(self.load_plugin_source(plugin_source));

        self.lua.context(|lua_ctx| {
            let globals = lua_ctx.globals();

            let create_plugin: Function = lua_ctx.load(plugin_lua).eval()?;

            let plugin_options_table: Table = lua_ctx.load(&plugin_options).eval()?;
            let plugin_instance: Table = create_plugin.call(plugin_options_table)?;

            let plugin_name: String = plugin_instance.get("name")?;
            // if plugin_name.trim().is_empty() {
            //     return Err(rlua::Error::ExternalError(Arc::new(std::error::Error(
            //         "".to_string(),
            //     ))));
            // }
            log::trace!(
                "Loaded plugin '{}' from source: {}",
                plugin_name,
                plugin_source
            );

            let plugins_table: Table = globals.get("plugins")?;
            plugins_table.set(plugins_table.len()? + 1, plugin_instance)?;

            Ok::<(), rlua::Error>(())
        })
    }

    pub fn middleware(&self, id: &str) -> Result<Option<SnapshotMiddleware>, rlua::Error> {
        self.lua.context(|lua_ctx| {
            let globals = lua_ctx.globals();

            let plugins: Table = globals.get("plugins")?;
            for plugin in plugins.sequence_values::<Table>() {
                let middleware_fn: Function = plugin?.get("middleware")?;
                let middleware_str: Option<String> = middleware_fn.call(id)?;
                let middleware_enum = match middleware_str {
                    Some(str) => SnapshotMiddleware::from_str(&str).ok(),
                    None => None,
                };
                if middleware_enum.is_some() {
                    return Ok(middleware_enum);
                }
            }

            Ok(None)
        })
    }

    pub fn load(&self, id: &str, contents: &str) -> Result<Option<String>, rlua::Error> {
        self.lua.context(|lua_ctx| {
            let globals = lua_ctx.globals();

            let plugins: Table = globals.get("plugins")?;
            for plugin in plugins.sequence_values::<Table>() {
                let load_fn: Function = plugin?.get("load")?;
                let load_str: Option<String> = load_fn.call((id, contents))?;
                if load_str.is_some() {
                    return Ok(load_str);
                }
            }

            Ok(None)
        })
    }
}
