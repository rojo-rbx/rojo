use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
};

use log::warn;
use rbx_dom_weak::UnresolvedRbxValue;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

pub static PROJECT_FILENAME: &str = "default.project.json";

/// Error type returned by any function that handles projects.
#[derive(Debug, Snafu)]
pub struct ProjectError(Error);

#[derive(Debug, Snafu)]
enum Error {
    /// In cases where we're trying to create a new project, this happens if the
    /// project file already exists.
    AlreadyExists { path: PathBuf },

    /// A general IO error occurred.
    Io { source: io::Error, path: PathBuf },

    /// An error with JSON parsing occurred.
    Json {
        source: serde_json::Error,
        path: PathBuf,
    },
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

    pub fn init_place(_project_fuzzy_path: &Path) -> Result<PathBuf, ProjectError> {
        unimplemented!();
    }

    pub fn init_model(_project_fuzzy_path: &Path) -> Result<PathBuf, ProjectError> {
        unimplemented!();
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

    pub fn load_fuzzy(fuzzy_project_location: &Path) -> Result<Option<Self>, ProjectError> {
        if let Some(project_path) = Self::locate(fuzzy_project_location) {
            let project = Self::load_exact(&project_path)?;

            Ok(Some(project))
        } else {
            Ok(None)
        }
    }

    fn load_exact(project_file_location: &Path) -> Result<Self, ProjectError> {
        let contents = fs::read_to_string(project_file_location).context(Io {
            path: project_file_location,
        })?;

        let mut project: Project = serde_json::from_str(&contents).context(Json {
            path: project_file_location,
        })?;

        project.file_location = project_file_location.to_path_buf();
        project.check_compatibility();

        Ok(project)
    }

    pub fn save(&self) -> Result<(), ProjectError> {
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
