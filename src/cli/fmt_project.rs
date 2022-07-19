use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use crate::project::Project;

use super::resolve_path;

/// Reformat a Rojo project using the standard JSON formatting rules.
#[derive(Debug, Parser)]
pub struct FmtProjectCommand {
    /// Path to the project to format. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,
}

impl FmtProjectCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let base_path = resolve_path(&self.project);
        let project = Project::load_fuzzy(&base_path)?
            .context("A project file is required to run 'rojo fmt-project'")?;

        let serialized = serde_json::to_string_pretty(&project)
            .context("could not re-encode project file as JSON")?;

        fs_err::write(&project.file_location, &serialized)
            .context("could not write back to project file")?;

        Ok(())
    }
}
