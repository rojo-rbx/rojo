use memofs::Vfs;
use std::{path::Path, sync::Arc};

use crate::plugin_env::PluginEnv;

pub fn load_file(
    vfs: &Vfs,
    plugin_env: &PluginEnv,
    path: &Path,
) -> Result<Arc<Vec<u8>>, anyhow::Error> {
    let plugin_result = plugin_env.load(path.to_str().unwrap());
    match plugin_result {
        Ok(Some(data)) => return Ok(Arc::new(data.as_bytes().to_vec())),
        Ok(None) => {}
        Err(_) => {}
    }

    let contents = vfs.read(path)?;
    return Ok(contents);
}
