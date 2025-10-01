use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};
use std::{
    ffi::OsStr,
    io::{self, Write},
};

use anyhow::{bail, format_err};
use clap::Parser;
use fs_err as fs;
use fs_err::OpenOptions;
use memofs::{InMemoryFs, Vfs, VfsSnapshot};

use super::resolve_path;

const GIT_IGNORE_PLACEHOLDER: &str = "gitignore.txt";

static TEMPLATE_BINCODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/templates.bincode"));

/// Initializes a new Rojo project.
///
/// By default, this will attempt to initialize a 'git' repository in the
/// project directory if `git` is installed. To avoid this, pass `--skip-git`.
#[derive(Debug, Parser)]
pub struct InitCommand {
    /// Path to the place to create the project. Defaults to the current directory.
    #[clap(default_value = "")]
    pub path: PathBuf,

    /// The kind of project to create, 'place', 'plugin', or 'model'.
    #[clap(long, default_value = "place")]
    pub kind: InitKind,

    /// Skips the initialization of a git repository.
    #[clap(long)]
    pub skip_git: bool,
}

impl InitCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let template = self.kind.template();

        let base_path = resolve_path(&self.path);
        fs::create_dir_all(&base_path)?;

        let canonical = fs::canonicalize(&base_path)?;
        let project_name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("new-project");

        let project_params = ProjectParams {
            name: project_name.to_owned(),
        };

        println!(
            "Creating new {:?} project '{}'",
            self.kind, project_params.name
        );

        let vfs = Vfs::new(template);
        vfs.set_watch_enabled(false);

        let mut queue = VecDeque::with_capacity(8);
        for entry in vfs.read_dir("")? {
            queue.push_back(entry?.path().to_path_buf())
        }

        while let Some(mut path) = queue.pop_front() {
            let metadata = vfs.metadata(&path)?;
            if metadata.is_dir() {
                fs_err::create_dir(base_path.join(&path))?;
                for entry in vfs.read_dir(&path)? {
                    queue.push_back(entry?.path().to_path_buf());
                }
            } else {
                let content = vfs.read_to_string_lf_normalized(&path)?;
                if let Some(file_stem) = path.file_name().and_then(OsStr::to_str) {
                    if file_stem == GIT_IGNORE_PLACEHOLDER && !self.skip_git {
                        path.set_file_name(".gitignore");
                    }
                }
                write_if_not_exists(
                    &base_path.join(&path),
                    &project_params.render_template(&content),
                )?;
            }
        }

        if !self.skip_git && should_git_init(&base_path) {
            log::debug!("Initializing Git repository...");

            let status = Command::new("git")
                .arg("init")
                .current_dir(&base_path)
                .status()?;

            if !status.success() {
                bail!("git init failed: status code {:?}", status.code());
            }
        }

        println!("Created project successfully.");

        Ok(())
    }
}

/// The templates we support for initializing a Rojo project.
#[derive(Debug, Clone, Copy)]
pub enum InitKind {
    /// A place that contains a baseplate.
    Place,

    /// An empty model, suitable for a library.
    Model,

    /// An empty plugin.
    Plugin,
}

impl InitKind {
    fn template(&self) -> InMemoryFs {
        let template_path = match self {
            Self::Place => "place",
            Self::Model => "model",
            Self::Plugin => "plugin",
        };

        let snapshot: VfsSnapshot = bincode::deserialize(TEMPLATE_BINCODE)
            .expect("Rojo's templates were not properly packed into Rojo's binary");

        if let VfsSnapshot::Dir { mut children } = snapshot {
            if let Some(template) = children.remove(template_path) {
                let mut fs = InMemoryFs::new();
                fs.load_snapshot("", template)
                    .expect("loading a template in memory should never fail");
                fs
            } else {
                panic!("template for project type {:?} is missing", self)
            }
        } else {
            panic!("Rojo's templates were packed as a file instead of a directory")
        }
    }
}

impl FromStr for InitKind {
    type Err = anyhow::Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "place" => Ok(InitKind::Place),
            "model" => Ok(InitKind::Model),
            "plugin" => Ok(InitKind::Plugin),
            _ => Err(format_err!(
                "Invalid init kind '{}'. Valid kinds are: place, model, plugin",
                source
            )),
        }
    }
}

/// Contains parameters used in templates to create a project.
struct ProjectParams {
    name: String,
}

impl ProjectParams {
    /// Render a template by replacing variables with project parameters.
    fn render_template(&self, template: &str) -> String {
        template
            .replace("{project_name}", &self.name)
            .replace("{rojo_version}", env!("CARGO_PKG_VERSION"))
    }
}

/// Tells whether we should initialize a Git repository inside the given path.
///
/// Will return false if the user doesn't have Git installed or if the path is
/// already inside a Git repository.
fn should_git_init(path: &Path) -> bool {
    let result = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(path)
        .status();

    match result {
        // If the command ran, but returned a non-zero exit code, we are not in
        // a Git repo and we should initialize one.
        Ok(status) => !status.success(),

        // If the command failed to run, we probably don't have Git installed.
        Err(_) => false,
    }
}

/// Write a file if it does not exist yet, otherwise, leave it alone.
fn write_if_not_exists(path: &Path, contents: &str) -> Result<(), anyhow::Error> {
    let file_res = OpenOptions::new().write(true).create_new(true).open(path);

    let mut file = match file_res {
        Ok(file) => file,
        Err(err) => {
            return match err.kind() {
                io::ErrorKind::AlreadyExists => return Ok(()),
                _ => Err(err.into()),
            }
        }
    };

    file.write_all(contents.as_bytes())?;

    Ok(())
}
