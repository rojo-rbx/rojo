use mlua::{Lua, Result, Table, String as LuaString};
use std::path::Path;
use std::sync::OnceLock; // Used for thread-safe, one-time initialization

// A static variable to hold the initialized Lua VM instance.
// OnceLock ensures the VM is created only once and is safe to use across threads.
static LUA_VM: OnceLock<Lua> = OnceLock::new();

/// Initializes the Lua VM and configures the Prometheus pipeline.
/// This function must be called once when the Rojo application starts.
pub fn initialize_prometheus() -> Result<()> {
    // Create a new independent Lua interpreter instance.
    let lua = Lua::new();

    // Define the relative path to the Prometheus library within your repo.
    let prometheus_dir = Path::new("./libraries/Prometheus");

    lua.context(|lua_ctx| {
        // Get the global table to modify 'package.path'.
        let globals = lua_ctx.globals();
        let package_table: Table = globals.get("package")?;
        let current_path: LuaString = package_table.get("path")?;

        // Format the new path string, adding Prometheus's src directories to Lua's search paths.
        let new_path = format!(
            "{}/src/?.lua;{}/src/?/init.lua;{}",
            prometheus_dir.display(),
            prometheus_dir.display(),
            current_path.to_str()?
        );
        package_table.set("path", new_path)?;

        // Load and configure the Pipeline within the Lua VM context.
        lua_ctx.load(r#"
            local Pipeline = require("pipeline")
            
            pipeline = Pipeline:fromConfig{
                LuaVersion = "LuaU",
                PrettyPrint = false,
                Seed = math.random(0, 2^31-1),
                Steps = {
                    "Cleaner",
                    "IdentifierMangling",
                    "ConstantEncryption",
                    "ControlFlowFlattening",
                    "DeadCodeInsertion"
                }
            }
        "#).exec()?;
        
        Ok(())
    })?;

    // Store the initialized VM in our static variable.
    LUA_VM.set(lua).map_err(|_| mlua::Error::runtime("Failed to set LUA_VM OnceLock"))?;

    Ok(())
}


/// Obfuscates a given Lua source code string using the initialized Prometheus pipeline.
///
/// # Arguments
/// * `source_code` - The clean Lua code to obfuscate.
pub fn obfuscate(source_code: &str) -> Result<String> {
    // Access the previously initialized VM instance. Panics if initialize_prometheus() wasn't called.
    let lua = LUA_VM.get().expect("Prometheus was not initialized! Call initialize_prometheus() first.");

    // Use the context to execute Lua code and call the pipeline's apply method.
    lua.context(|lua_ctx| {
        let obfuscated: String = lua_ctx
            .load(r#"return pipeline:apply(...)"#)
            .call((source_code, "script.lua"))?;
    
        // Optional: print the result to the console for debugging within Rojo CLI
        // println!("Obfuscated Lua:\n{}", obfuscated);
        
        Ok(obfuscated)
    })
}

/// Example usage in a main function (for testing purposes).
fn main() -> Result<()> {
    // 1. Initialize Prometheus once at application start.
    initialize_prometheus()?;
    
    Ok(())
}
