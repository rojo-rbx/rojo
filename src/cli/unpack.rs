use std::{io::BufReader, path::PathBuf};

use anyhow::bail;
use clap::Parser;
use fs_err::File;
use rbx_dom_weak::{Instance, WeakDom};

use crate::{Project, ProjectNode};

use super::resolve_path;

/// Unpack a Roblox place file into an existing Rojo project.
#[derive(Debug, Parser)]
pub struct UnpackCommand {
    /// Path to the project to unpack. Defaults to the current directory.
    #[clap(long, default_value = "")]
    pub project: PathBuf,

    /// Path to the place to unpack from.
    pub place: PathBuf,
}

impl UnpackCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);
        let project = match Project::load_fuzzy(&project_path)? {
            Some(project) => project,
            None => bail!("No project file was found; rojo unpack requires a project file."),
        };

        let place_ext = self
            .place
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        let file = BufReader::new(File::open(&self.place)?);

        let dom = match place_ext.as_deref() {
            Some("rbxl") => rbx_binary::from_reader(file)?,
            Some("rbxlx") => rbx_xml::from_reader_default(file)?,
            Some(_) | None => bail!("Place files must end in .rbxl or .rbxlx"),
        };

        let context = Context { project, dom };
        context.unpack();

        Ok(())
    }
}

struct Context {
    project: Project,
    dom: WeakDom,
}

impl Context {
    fn unpack(&self) {
        self.unpack_node(&self.project.tree, self.dom.root());
    }

    fn unpack_node(&self, node: &ProjectNode, instance: &Instance) {
        // TODO
    }
}
