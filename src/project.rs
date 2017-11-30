use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use serde_json;

pub static PROJECT_FILENAME: &'static str = "rojo.json";

#[derive(Debug)]
pub enum ProjectLoadError {
    DidNotExist,
    FailedToOpen,
    FailedToRead,
    Invalid,
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
            &ProjectInitError::FailedToCreate | &ProjectInitError::FailedToWrite => {
                write!(f, "Failed to write to the given location.")
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPartition {
    pub path: String,
    pub target: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub serve_port: u64,
    pub partitions: HashMap<String, ProjectPartition>,
}

impl Project {
    pub fn new<T: Into<String>>(name: T) -> Project {
        Project {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn init<T: AsRef<Path>>(location: T) -> Result<Project, ProjectInitError> {
        let location = location.as_ref();
        let package_path = location.join(PROJECT_FILENAME);

        match fs::metadata(&package_path) {
            Ok(_) => return Err(ProjectInitError::AlreadyExists),
            Err(_) => {},
        }

        let mut file = match File::create(&package_path) {
            Ok(f) => f,
            Err(_) => return Err(ProjectInitError::FailedToCreate),
        };

        let name = match location.file_name() {
            Some(v) => v.to_string_lossy().into_owned(),
            None => "new-project".to_string(),
        };

        let project = Project::new(name);
        let serialized = serde_json::to_string_pretty(&project).unwrap();

        match file.write(serialized.as_bytes()) {
            Ok(_) => {},
            Err(_) => return Err(ProjectInitError::FailedToWrite),
        }

        Ok(project)
    }

    pub fn load<T: AsRef<Path>>(location: T) -> Result<Project, ProjectLoadError> {
        let package_path = location.as_ref().join(Path::new(PROJECT_FILENAME));

        match fs::metadata(&package_path) {
            Ok(_) => {},
            Err(_) => return Err(ProjectLoadError::DidNotExist),
        }

        let mut file = match File::open(&package_path) {
            Ok(f) => f,
            Err(_) => return Err(ProjectLoadError::FailedToOpen),
        };

        let mut contents = String::new();

        match file.read_to_string(&mut contents) {
            Ok(_) => {},
            Err(_) => return Err(ProjectLoadError::FailedToRead),
        }

        match serde_json::from_str(&contents) {
            Ok(v) => Ok(v),
            Err(_) => return Err(ProjectLoadError::Invalid),
        }
    }

    pub fn save<T: AsRef<Path>>(&self, location: T) -> Result<(), ProjectSaveError> {
        let package_path = location.as_ref().join(Path::new(PROJECT_FILENAME));

        let mut file = match File::create(&package_path) {
            Ok(f) => f,
            Err(_) => return Err(ProjectSaveError::FailedToCreate),
        };

        let serialized = serde_json::to_string_pretty(self).unwrap();

        file.write(serialized.as_bytes()).unwrap();

        Ok(())
    }
}

impl Default for Project {
    fn default() -> Project {
        Project {
            name: "some-project".to_string(),
            serve_port: 8000,
            partitions: HashMap::new(),
        }
    }
}
