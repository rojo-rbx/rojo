use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rand::{self, Rng};

use serde_json;

use partition::Partition;

pub static PROJECT_FILENAME: &'static str = "rojo.json";

#[derive(Debug)]
pub enum ProjectLoadError {
    DidNotExist(PathBuf),
    FailedToOpen(PathBuf),
    FailedToRead(PathBuf),
    InvalidJson(PathBuf, serde_json::Error),
}

impl fmt::Display for ProjectLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ProjectLoadError::InvalidJson(ref project_path, ref serde_err) => {
                write!(f, "Found invalid JSON reading project: {}\nError: {}", project_path.display(), serde_err)
            },
            &ProjectLoadError::FailedToOpen(ref project_path) |
            &ProjectLoadError::FailedToRead(ref project_path) => {
                write!(f, "Found project file, but failed to read it: {}", project_path.display())
            },
            &ProjectLoadError::DidNotExist(ref project_path) => {
                write!(f, "Could not locate a project file at {}.\nUse 'rojo init' to create one.", project_path.display())
            },
        }
    }
}

#[derive(Debug)]
pub enum ProjectSaveError {
    FailedToCreate,
}

#[derive(Debug)]
pub enum ProjectInitError {
    AlreadyExists,
    FailedToCreate,
    FailedToWrite,
}

impl fmt::Display for ProjectInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ProjectInitError::AlreadyExists => {
                write!(f, "A project already exists at that location.")
            },
            &ProjectInitError::FailedToCreate |
            &ProjectInitError::FailedToWrite => {
                write!(f, "Failed to write to the given location.")
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProjectPartition {
    /// A slash-separated path to a file or folder, relative to the project's
    /// directory.
    pub path: String,

    /// A dot-separated route to a Roblox instance, relative to game.
    pub target: String,
}

/// Represents a Rojo project in the format that's most convenient for users to
/// edit. This should generally line up with `Project`, but can diverge when
/// there's either compatibility shims or when the data structures that Rojo
/// want are too verbose to write in JSON but easy to convert from something
/// else.
//
/// Holds anything that can be configured with `rojo.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SourceProject {
    pub name: String,
    pub serve_port: u64,
    pub partitions: HashMap<String, SourceProjectPartition>,
}

impl Default for SourceProject {
    fn default() -> SourceProject {
        SourceProject {
            name: "new-project".to_string(),
            serve_port: 8000,
            partitions: HashMap::new(),
        }
    }
}

/// Represents a Rojo project in the format that's convenient for Rojo to work
/// with.
#[derive(Debug, Clone)]
pub struct Project {
    /// The path to the project file that this project is associated with.
    pub project_path: PathBuf,

    /// The name of this project, used for user-facing labels.
    pub name: String,

    /// The port that this project will run a web server on.
    pub serve_port: u64,

    /// All of the project's partitions, laid out in an expanded way.
    pub partitions: HashMap<String, Partition>,
}

impl Project {
    fn from_source_project(source_project: SourceProject, project_path: PathBuf) -> Project {
        let mut partitions = HashMap::new();

        {
            let project_directory = project_path.parent().unwrap();

            for (partition_name, partition) in source_project.partitions.into_iter() {
                let path = project_directory.join(&partition.path);
                let target = partition.target
                    .split(".")
                    .map(String::from)
                    .collect::<Vec<_>>();

                partitions.insert(partition_name.clone(), Partition {
                    path,
                    target,
                    name: partition_name,
                });
            }
        }

        Project {
            project_path,
            name: source_project.name,
            serve_port: source_project.serve_port,
            partitions,
        }
    }

    fn as_source_project(&self) -> SourceProject {
        let mut partitions = HashMap::new();

        for partition in self.partitions.values() {
            let path = partition.path.strip_prefix(&self.project_path)
                .unwrap_or_else(|_| &partition.path)
                .to_str()
                .unwrap()
                .to_string();

            let target = partition.target.join(".");

            partitions.insert(partition.name.clone(), SourceProjectPartition {
                path,
                target,
            });
        }

        SourceProject {
            partitions,
            name: self.name.clone(),
            serve_port: self.serve_port,
        }
    }

    /// Initializes a new project inside the given folder path.
    pub fn init<T: AsRef<Path>>(location: T) -> Result<Project, ProjectInitError> {
        let location = location.as_ref();
        let project_path = location.join(PROJECT_FILENAME);

        // We abort if the project file already exists.
        fs::metadata(&project_path)
            .map_err(|_| ProjectInitError::AlreadyExists)?;

        let mut file = File::create(&project_path)
            .map_err(|_| ProjectInitError::FailedToCreate)?;

        // Try to give the project a meaningful name.
        // If we can't, we'll just fall back to a default.
        let name = match location.file_name() {
            Some(v) => v.to_string_lossy().into_owned(),
            None => "new-project".to_string(),
        };

        // Generate a random port to run the server on.
        let serve_port = rand::thread_rng().gen_range(2000, 49151);

        // Configure the project with all of the values we know so far.
        let source_project = SourceProject {
            name,
            serve_port,
            partitions: HashMap::new(),
        };
        let serialized = serde_json::to_string_pretty(&source_project).unwrap();

        file.write(serialized.as_bytes())
            .map_err(|_| ProjectInitError::FailedToWrite)?;

        Ok(Project::from_source_project(source_project, project_path))
    }

    /// Attempts to load a project from the file named PROJECT_FILENAME from the
    /// given folder.
    pub fn load<T: AsRef<Path>>(location: T) -> Result<Project, ProjectLoadError> {
        let project_path = location.as_ref().join(Path::new(PROJECT_FILENAME));

        fs::metadata(&project_path)
            .map_err(|_| ProjectLoadError::DidNotExist(project_path.clone()))?;

        let mut file = File::open(&project_path)
            .map_err(|_| ProjectLoadError::FailedToOpen(project_path.clone()))?;

        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .map_err(|_| ProjectLoadError::FailedToRead(project_path.clone()))?;

        let source_project = serde_json::from_str(&contents)
            .map_err(|e| ProjectLoadError::InvalidJson(project_path.clone(), e))?;

        Ok(Project::from_source_project(source_project, project_path))
    }

    /// Saves the given project file to the given folder with the appropriate name.
    pub fn save<T: AsRef<Path>>(&self, location: T) -> Result<(), ProjectSaveError> {
        let project_path = location.as_ref().join(Path::new(PROJECT_FILENAME));

        let mut file = File::create(&project_path)
            .map_err(|_| ProjectSaveError::FailedToCreate)?;

        let source_project = self.as_source_project();
        let serialized = serde_json::to_string_pretty(&source_project).unwrap();

        file.write(serialized.as_bytes()).unwrap();

        Ok(())
    }
}
