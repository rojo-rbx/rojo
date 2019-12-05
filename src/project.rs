use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, fs, io,
    path::{Path, PathBuf},
};

use failure::Fail;
use log::warn;
use rbx_dom_weak::UnresolvedRbxValue;
use serde::{Deserialize, Serialize};

static DEFAULT_PLACE: &str = include_str!("../assets/place.project.json");

pub static PROJECT_FILENAME: &str = "default.project.json";

#[derive(Debug, Fail)]
pub enum ProjectLoadError {
    NotFound,

    Io {
        #[fail(cause)]
        inner: io::Error,
        path: PathBuf,
    },

    Json {
        #[fail(cause)]
        inner: serde_json::Error,
        path: PathBuf,
    },
}

impl fmt::Display for ProjectLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use self::ProjectLoadError::*;

        match self {
            NotFound => write!(formatter, "Project file not found"),
            Io { inner, path } => {
                write!(formatter, "I/O error: {} in path {}", inner, path.display())
            }
            Json { inner, path } => write!(
                formatter,
                "JSON error: {} in path {}",
                inner,
                path.display()
            ),
        }
    }
}

/// Error returned by Project::init_place and Project::init_model
#[derive(Debug, Fail)]
pub enum ProjectInitError {
    AlreadyExists(PathBuf),
    IoError(#[fail(cause)] io::Error),
    SaveError(#[fail(cause)] ProjectSaveError),
    JsonError(#[fail(cause)] serde_json::Error),
}

impl fmt::Display for ProjectInitError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProjectInitError::AlreadyExists(path) => {
                write!(output, "Path {} already exists", path.display())
            }
            ProjectInitError::IoError(inner) => write!(output, "IO error: {}", inner),
            ProjectInitError::SaveError(inner) => write!(output, "{}", inner),
            ProjectInitError::JsonError(inner) => write!(output, "{}", inner),
        }
    }
}

/// Error returned by Project::save
#[derive(Debug, Fail)]
pub enum ProjectSaveError {
    #[fail(display = "JSON error: {}", _0)]
    JsonError(#[fail(cause)] serde_json::Error),

    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectNode {
    #[serde(rename = "$className", skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    #[serde(flatten)]
    pub children: BTreeMap<String, ProjectNode>,

    #[serde(
        rename = "$properties",
        default,
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub properties: HashMap<String, UnresolvedRbxValue>,

    #[serde(
        rename = "$ignoreUnknownInstances",
        skip_serializing_if = "Option::is_none"
    )]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(
        rename = "$path",
        serialize_with = "crate::path_serializer::serialize_option_absolute",
        skip_serializing_if = "Option::is_none"
    )]
    pub path: Option<PathBuf>,
}

impl ProjectNode {
    fn validate_reserved_names(&self) {
        for (name, child) in &self.children {
            if name.starts_with('$') {
                warn!(
                    "Keys starting with '$' are reserved by Rojo to ensure forward compatibility."
                );
                warn!(
                    "This project uses the key '{}', which should be renamed.",
                    name
                );
            }

            child.validate_reserved_names();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub tree: ProjectNode,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_port: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_place_ids: Option<HashSet<u64>>,

    #[serde(skip)]
    pub file_location: PathBuf,
}

impl Project {
    pub fn is_project_file(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".project.json"))
            .unwrap_or(false)
    }

    pub fn init_place(project_fuzzy_path: &Path) -> Result<PathBuf, ProjectInitError> {
        let project_path = Project::pick_path_for_init(project_fuzzy_path)?;

        let project_name = if project_fuzzy_path == project_path {
            project_fuzzy_path
                .parent()
                .expect("Path did not have a parent directory")
                .file_name()
                .expect("Path did not have a file name")
                .to_str()
                .expect("Path had invalid Unicode")
        } else {
            project_fuzzy_path
                .file_name()
                .expect("Path did not have a file name")
                .to_str()
                .expect("Path had invalid Unicode")
        };

        let mut project = Project::load_from_slice(DEFAULT_PLACE.as_bytes(), &project_path)
            .map_err(ProjectInitError::JsonError)?;

        project.name = project_name.to_owned();

        project.save().map_err(ProjectInitError::SaveError)?;

        Ok(project_path)
    }

    pub fn init_model(project_fuzzy_path: &Path) -> Result<PathBuf, ProjectInitError> {
        let project_path = Project::pick_path_for_init(project_fuzzy_path)?;

        let project_name = if project_fuzzy_path == project_path {
            project_fuzzy_path
                .parent()
                .expect("Path did not have a parent directory")
                .file_name()
                .expect("Path did not have a file name")
                .to_str()
                .expect("Path had invalid Unicode")
        } else {
            project_fuzzy_path
                .file_name()
                .expect("Path did not have a file name")
                .to_str()
                .expect("Path had invalid Unicode")
        };

        let project_folder_path = project_path
            .parent()
            .expect("Path did not have a parent directory");

        let tree = ProjectNode {
            path: Some(project_folder_path.join("src")),
            ..Default::default()
        };

        let project = Project {
            name: project_name.to_string(),
            tree,
            serve_port: None,
            serve_place_ids: None,
            file_location: project_path.clone(),
        };

        project.save().map_err(ProjectInitError::SaveError)?;

        Ok(project_path)
    }

    fn pick_path_for_init(project_fuzzy_path: &Path) -> Result<PathBuf, ProjectInitError> {
        let is_exact = project_fuzzy_path.extension().is_some();

        let project_path = if is_exact {
            project_fuzzy_path.to_path_buf()
        } else {
            project_fuzzy_path.join(PROJECT_FILENAME)
        };

        match fs::metadata(&project_path) {
            Err(error) => match error.kind() {
                io::ErrorKind::NotFound => {}
                _ => return Err(ProjectInitError::IoError(error)),
            },
            Ok(_) => return Err(ProjectInitError::AlreadyExists(project_path)),
        }

        Ok(project_path)
    }

    /// Attempt to locate a project represented by the given path.
    ///
    /// This will find a project if the path refers to a `.project.json` file,
    /// or is a folder that contains a `default.project.json` file.
    fn locate(path: &Path) -> Option<PathBuf> {
        let meta = fs::metadata(path).ok()?;

        if meta.is_file() {
            if Project::is_project_file(path) {
                Some(path.to_path_buf())
            } else {
                None
            }
        } else {
            let child_path = path.join(PROJECT_FILENAME);
            let child_meta = fs::metadata(&child_path).ok()?;

            if child_meta.is_file() {
                Some(child_path)
            } else {
                // This is a folder with the same name as a Rojo default project
                // file.
                //
                // That's pretty weird, but we can roll with it.
                None
            }
        }
    }

    pub fn load_from_slice(
        contents: &[u8],
        project_file_location: &Path,
    ) -> Result<Self, serde_json::Error> {
        let mut project: Self = serde_json::from_slice(&contents)?;
        project.file_location = project_file_location.to_path_buf();
        project.check_compatibility();
        Ok(project)
    }

    pub fn load_fuzzy(fuzzy_project_location: &Path) -> Result<Self, ProjectLoadError> {
        if let Some(project_path) = Self::locate(fuzzy_project_location) {
            Self::load_exact(&project_path)
        } else {
            Err(ProjectLoadError::NotFound)
        }
    }

    fn load_exact(project_file_location: &Path) -> Result<Self, ProjectLoadError> {
        let contents =
            fs::read_to_string(project_file_location).map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => ProjectLoadError::NotFound,
                _ => ProjectLoadError::Io {
                    inner: error,
                    path: project_file_location.to_path_buf(),
                },
            })?;

        let mut project: Project =
            serde_json::from_str(&contents).map_err(|inner| ProjectLoadError::Json {
                inner,
                path: project_file_location.to_path_buf(),
            })?;

        project.file_location = project_file_location.to_path_buf();
        project.check_compatibility();

        Ok(project)
    }

    pub fn save(&self) -> Result<(), ProjectSaveError> {
        unimplemented!()
    }

    /// Checks if there are any compatibility issues with this project file and
    /// warns the user if there are any.
    fn check_compatibility(&self) {
        self.tree.validate_reserved_names();
    }

    pub fn folder_location(&self) -> &Path {
        self.file_location.parent().unwrap()
    }
}
