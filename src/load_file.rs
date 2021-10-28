use anyhow::Context;
use memofs::Vfs;
use std::{path::Path, str, sync::Arc};

use crate::plugin_env::PluginEnv;

pub fn load_file(
    vfs: &Vfs,
    plugin_env: &PluginEnv,
    path: &Path,
) -> Result<Arc<Vec<u8>>, anyhow::Error> {
    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?;

    let plugin_result = plugin_env.load(path.to_str().unwrap(), contents_str);
    match plugin_result {
        Ok(Some(data)) => return Ok(Arc::new(data.as_bytes().to_vec())),
        Ok(None) => {}
        Err(_) => {}
    }

    return Ok(contents);
}
