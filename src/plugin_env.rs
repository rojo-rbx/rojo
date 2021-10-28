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

            let create_plugin_fn: Option<Function> =
                lua_ctx.load(plugin_lua).set_name(plugin_source)?.eval()?;
            let create_plugin_fn = match create_plugin_fn {
                Some(v) => v,
                None => {
                    return Err(rlua::Error::RuntimeError(
                        format!(
                            "plugin from source '{}' did not return a creation function.",
                            plugin_source
                        )
                        .to_string(),
                    ))
                }
            };

            let plugin_options_table: Table = lua_ctx
                .load(&plugin_options)
                .set_name("plugin options")?
                .eval()?;

            let plugin_instance: Option<Table> = create_plugin_fn.call(plugin_options_table)?;
            let plugin_instance = match plugin_instance {
                Some(v) => v,
                None => {
                    return Err(rlua::Error::RuntimeError(
                        format!(
                            "creation function for plugin from source '{}' did not return a plugin instance.",
                            plugin_source
                        )
                        .to_string(),
                    ))
                }
            };

            let plugin_name: Option<String> = plugin_instance.get("name")?;
            let plugin_name = match plugin_name.unwrap_or("".to_owned()) {
                v if v.trim().is_empty() => {
                    return Err(rlua::Error::RuntimeError(
                        format!(
                            "plugin instance for plugin from source '{}' did not have a name.",
                            plugin_source
                        )
                        .to_string(),
                    ))
                },
                v => v
            };

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
