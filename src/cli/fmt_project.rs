use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use memofs::Vfs;

use crate::project::Project;

use super::resolve_path;

/// Reformat a Rojo project in its existing file format.
#[derive(Debug, Parser)]
pub struct FmtProjectCommand {
    /// Path to the project to format. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,
}

impl FmtProjectCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let vfs = Vfs::new_default()?;
        vfs.set_watch_enabled(false);

        let base_path = resolve_path(&self.project)?;
        let project = Project::load_fuzzy(&vfs, &base_path)?
            .context("A project file is required to run 'rojo fmt-project'")?;

        let serialized = project
            .to_bytes()
            .context("could not re-encode project file")?;

        fs_err::write(&project.file_location, serialized)
            .context("could not write back to project file")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn format_yaml_and_toml_default_projects() {
        for (name, contents, formatted) in [
            (
                "default.project.yaml",
                "tree: { $className: Folder }",
                "tree:\n",
            ),
            (
                "default.project.toml",
                "tree = { '$className' = 'Folder' }",
                "[tree]\n",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join(name);
            fs_err::write(&file, contents).unwrap();

            FmtProjectCommand {
                project: dir.path().to_path_buf(),
            }
            .run()
            .unwrap();

            let output = fs_err::read_to_string(file).unwrap();
            assert!(output.contains(formatted), "{output}");
        }
    }
}
