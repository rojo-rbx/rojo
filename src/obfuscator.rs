use mlua::{Lua, prelude::LuaError, Result as LuaResult, Function, Value, Table};
use std::{sync::{OnceLock, Mutex}, cell::RefCell};

const PROMETHEUS_DIR: &str = "./libraries/Prometheus";

static LUA_VM: OnceLock<Mutex<RefCell<Lua>>> = OnceLock::new();

pub fn initialize_prometheus() -> LuaResult<()> {
    let lua = Lua::new();

    // update package.path
    {
        let globals = lua.globals();
        let package: Table = globals.get("package")?;
        let old_path: String = package.get("path")?;
        let new_path = format!(
            "{}/src/?.lua;{}/src/?/init.lua;{}",
            PROMETHEUS_DIR, PROMETHEUS_DIR, old_path
        );
        package.set("path", new_path)?;
    }

    // load pipeline
    lua.load(r#"
arg = {} -- FIX: mlua does not define 'arg', config.lua expects it

local ok, Pipeline = pcall(require, "prometheus.pipeline")
if not ok then
    error("Failed to require prometheus.pipeline: " .. tostring(Pipeline))
end

pipeline = Pipeline:fromConfig{
    LuaVersion = "LuaU",
    PrettyPrint = false,
    Seed = math.random(0, 2^31 - 1),

    Steps = {
        { Name = "EncryptStrings" },
        { Name = "ProxifyLocals" },
        { Name = "SplitStrings" },
        { Name = "ConstantArray" },
        { Name = "Vmify" },
        { Name = "WrapInFunction" },
    }
}
"#).exec()?;

    LUA_VM.set(Mutex::new(RefCell::new(lua)))
        .map_err(|_| LuaError::RuntimeError("failed to set lua vm".into()))?;

    Ok(())
}

pub fn obfuscate(source: &str, class_name: &str, name: &str) -> LuaResult<String> {
    let lua_cell = LUA_VM
        .get()
        .ok_or_else(|| LuaError::RuntimeError("Lua VM not initialized".into()))?
        .lock()
        .map_err(|_| LuaError::RuntimeError("mutex poisoned".into()))?;

    let lua = lua_cell.borrow();

    let globals = lua.globals();
    let pipeline: Table = match globals.get::<_, Value>("pipeline")? {
        Value::Table(t) => t,
        _ => return Err(LuaError::RuntimeError("pipeline not a table".into())),
    };

    let apply: Function = pipeline.get("apply")?;
    // let result: String = apply.call((pipeline, source, "script.lua"))?;

    // Ok(result)

    match apply.call::<_, String>((pipeline.clone(), source, "script.lua")) {
        Ok(result) => {
            // println!("Obfuscated: {}", result);
            Ok(result)
        },
        Err(LuaError::RuntimeError(msg)) => {
            // eprintln!("Lua error: {}", msg);
            eprintln!("Lua error. ClassName: [{:#?}], Name: [{:#?}]", class_name, name);
            Ok(format!("
--[[
    Obfuscation failed - LuaError
    ClassName: [{}]
    Name: [{}]
    Error: [
        {}
    ]
]]

{}", class_name, name, msg, source
))
        },
        Err(msg) => {
            // eprintln!("Other Lua error: {:?}", msg);
            eprintln!("Other Lua error. ClassName: [{:#?}], Name: [{:#?}]", class_name, name);
            Ok(format!("
--[[
    Obfuscation failed - Error
    ClassName: [{}]
    Name: [{}]
    Error: [
        {}
    ]
]]

{}", class_name, name, msg, source
))
        },
    }

}
