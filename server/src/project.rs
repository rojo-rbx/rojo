use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use log::warn;
use failure::Fail;
use maplit::hashmap;
use rbx_tree::RbxValue;
use serde_derive::{Serialize, Deserialize};

pub static PROJECT_FILENAME: &'static str = "default.project.json";
pub static COMPAT_PROJECT_FILENAME: &'static str = "roblox-project.json";

/// SourceProject is the format that users author projects on-disk. Since we
/// want to do things like transforming paths to be absolute before handing them
/// off to the rest of Rojo, we use this intermediate struct.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceProject {
    name: String,
    tree: SourceProjectNode,

    #[serde(skip_serializing_if = "Option::is_none")]
    serve_port: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    serve_place_ids: Option<HashSet<u64>>,
}

impl SourceProject {
    /// Consumes the SourceProject and yields a Project, ready for prime-time.
    pub fn into_project(self, project_file_location: &Path) -> Project {
        let tree = self.tree.into_project_node(project_file_location);

        Project {
            name: self.name,
            tree,
            serve_port: self.serve_port,
            serve_place_ids: self.serve_place_ids,
            file_location: PathBuf::from(project_file_location),
        }
    }
}

/// Similar to SourceProject, the structure of nodes in the project tree is
/// slightly different on-disk than how we want to handle them in the rest of
/// Rojo.
#[derive(Debug, Serialize, Deserialize)]
struct SourceProjectNode {
    #[serde(rename = "$className", skip_serializing_if = "Option::is_none")]
    class_name: Option<String>,

    #[serde(rename = "$properties", default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
    properties: HashMap<String, RbxValue>,

    #[serde(rename = "$ignoreUnknownInstances", skip_serializing_if = "Option::is_none")]
    ignore_unknown_instances: Option<bool>,

    #[serde(rename = "$path", skip_serializing_if = "Option::is_none")]
    path: Option<String>,

    #[serde(flatten)]
    children: HashMap<String, SourceProjectNode>,
}

impl SourceProjectNode {
    /// Consumes the SourceProjectNode and turns it into a ProjectNode.
    pub fn into_project_node(mut self, project_file_location: &Path) -> ProjectNode {
        let children = self.children.drain()
            .map(|(key, value)| (key, value.into_project_node(project_file_location)))
            .collect();

        // Make sure that paths are absolute, transforming them by adding the
        // project folder if they're not already absolute.
        let path = self.path.as_ref().map(|source_path| {
            if Path::new(source_path).is_absolute() {
                PathBuf::from(source_path)
            } else {
                let project_folder_location = project_file_location.parent().unwrap();
                project_folder_location.join(source_path)
            }
        });

        ProjectNode {
            class_name: self.class_name,
            properties: self.properties,
            ignore_unknown_instances: self.ignore_unknown_instances,
            path,
            children,
        }
    }
}

/// Error returned by Project::load_exact
#[derive(Debug, Fail)]
pub enum ProjectLoadExactError {
    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "JSON error: {}", _0)]
    JsonError(#[fail(cause)] serde_json::Error),
}

/// Error returned by Project::load_fuzzy
#[derive(Debug, Fail)]
pub enum ProjectLoadFuzzyError {
    #[fail(display = "Project not found")]
    NotFound,

    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "JSON error: {}", _0)]
    JsonError(#[fail(cause)] serde_json::Error),
}

impl From<ProjectLoadExactError> for ProjectLoadFuzzyError {
    fn from(error: ProjectLoadExactError) -> ProjectLoadFuzzyError {
        match error {
            ProjectLoadExactError::IoError(inner) => ProjectLoadFuzzyError::IoError(inner),
            ProjectLoadExactError::JsonError(inner) => ProjectLoadFuzzyError::JsonError(inner),
        }
    }
}

/// Error returned by Project::init_place and Project::init_model
#[derive(Debug, Fail)]
pub enum ProjectInitError {
    AlreadyExists(PathBuf),
    IoError(#[fail(cause)] io::Error),
    SaveError(#[fail(cause)] ProjectSaveError),
}

impl fmt::Display for ProjectInitError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProjectInitError::AlreadyExists(path) => write!(output, "Path {} already exists", path.display()),
            ProjectInitError::IoError(inner) => write!(output, "IO error: {}", inner),
            ProjectInitError::SaveError(inner) => write!(output, "{}", inner),
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
    pub class_name: Option<String>,
    pub children: HashMap<String, ProjectNode>,
    pub properties: HashMap<String, RbxValue>,
    pub ignore_unknown_instances: Option<bool>,
    pub path: Option<PathBuf>,
}

impl ProjectNode {
    fn to_source_node(&self, project_file_location: &Path) -> SourceProjectNode {
        let children = self.children.iter()
            .map(|(key, value)| (key.clone(), value.to_source_node(project_file_location)))
            .collect();

        // If paths are relative to the project file, transform them to look
        // Unixy and write relative paths instead.
        //
        // This isn't perfect, since it means that paths like .. will stay as
        // absolute paths and make projects non-portable. Fixing this probably
        // means keeping the paths relative in the project format and making
        // everywhere else in Rojo do the resolution locally.
        let path = self.path.as_ref().map(|path| {
            let project_folder_location = project_file_location.parent().unwrap();

            match path.strip_prefix(project_folder_location) {
                Ok(stripped) => stripped.to_str().unwrap().replace("\\", "/"),
                Err(_) => format!("{}", path.display()),
            }
        });

        SourceProjectNode {
            class_name: self.class_name.clone(),
            properties: self.properties.clone(),
            ignore_unknown_instances: self.ignore_unknown_instances,
            children,
            path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub tree: ProjectNode,
    pub serve_port: Option<u16>,
    pub serve_place_ids: Option<HashSet<u64>>,
    pub file_location: PathBuf,
}

impl Project {
    pub fn init_place(project_fuzzy_path: &Path) -> Result<PathBuf, ProjectInitError> {
        let project_path = Project::init_pick_path(project_fuzzy_path)?;
        let project_folder_path = project_path.parent().unwrap();
        let project_name = if project_fuzzy_path == project_path {
            project_fuzzy_path.parent().unwrap().file_name().unwrap().to_str().unwrap()
        } else {
            project_fuzzy_path.file_name().unwrap().to_str().unwrap()
        };

        let tree = ProjectNode {
            class_name: Some(String::from("DataModel")),
            children: hashmap! {
                String::from("ReplicatedStorage") => ProjectNode {
                    class_name: Some(String::from("ReplicatedStorage")),
                    children: hashmap! {
                        String::from("Source") => ProjectNode {
                            path: Some(project_folder_path.join("src")),
                            ..Default::default()
                        },
                    },
                    ..Default::default()
                },
                String::from("HttpService") => ProjectNode {
                    class_name: Some(String::from("HttpService")),
                    properties: hashmap! {
                        String::from("HttpEnabled") => RbxValue::Bool {
                            value: true,
                        },
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        let project = Project {
            name: project_name.to_string(),
            tree,
            serve_port: None,
            serve_place_ids: None,
            file_location: project_path.clone(),
        };

        project.save()
            .map_err(ProjectInitError::SaveError)?;

        Ok(project_path)
    }

    pub fn init_model(project_fuzzy_path: &Path) -> Result<PathBuf, ProjectInitError> {
        let project_path = Project::init_pick_path(project_fuzzy_path)?;
        let project_folder_path = project_path.parent().unwrap();
        let project_name = if project_fuzzy_path == project_path {
            project_fuzzy_path.parent().unwrap().file_name().unwrap().to_str().unwrap()
        } else {
            project_fuzzy_path.file_name().unwrap().to_str().unwrap()
        };

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

        project.save()
            .map_err(ProjectInitError::SaveError)?;

        Ok(project_path)
    }

    fn init_pick_path(project_fuzzy_path: &Path) -> Result<PathBuf, ProjectInitError> {
        let is_exact = project_fuzzy_path.extension().is_some();

        let project_path = if is_exact {
            project_fuzzy_path.to_path_buf()
        } else {
            project_fuzzy_path.join(PROJECT_FILENAME)
        };

        match fs::metadata(&project_path) {
            Err(error) => match error.kind() {
                io::ErrorKind::NotFound => {},
                _ => return Err(ProjectInitError::IoError(error)),
            },
            Ok(_) => return Err(ProjectInitError::AlreadyExists(project_path)),
        }

        Ok(project_path)
    }

    pub fn locate(start_location: &Path) -> Option<PathBuf> {
        // TODO: Check for specific error kinds, convert 'not found' to Result.
        let location_metadata = fs::metadata(start_location).ok()?;

        // If this is a file, assume it's the config the user was looking for.
        if location_metadata.is_file() {
            return Some(start_location.to_path_buf());
        } else if location_metadata.is_dir() {
            let with_file = start_location.join(PROJECT_FILENAME);

            if let Ok(file_metadata) = fs::metadata(&with_file) {
                if file_metadata.is_file() {
                    return Some(with_file);
                }
            }

            let with_compat_file = start_location.join(COMPAT_PROJECT_FILENAME);

            if let Ok(file_metadata) = fs::metadata(&with_compat_file) {
                if file_metadata.is_file() {
                    return Some(with_compat_file);
                }
            }
        }

        match start_location.parent() {
            Some(parent_location) => Self::locate(parent_location),
            None => None,
        }
    }

    pub fn load_fuzzy(fuzzy_project_location: &Path) -> Result<Project, ProjectLoadFuzzyError> {
        let project_path = Self::locate(fuzzy_project_location)
            .ok_or(ProjectLoadFuzzyError::NotFound)?;

        Self::load_exact(&project_path).map_err(From::from)
    }

    pub fn load_exact(project_file_location: &Path) -> Result<Project, ProjectLoadExactError> {
        let contents = fs::read_to_string(project_file_location)
            .map_err(ProjectLoadExactError::IoError)?;

        let parsed: SourceProject = serde_json::from_str(&contents)
            .map_err(ProjectLoadExactError::JsonError)?;

        Ok(parsed.into_project(project_file_location))
    }

    pub fn save(&self) -> Result<(), ProjectSaveError> {
        let source_project = self.to_source_project();
        let mut file = File::create(&self.file_location)
            .map_err(ProjectSaveError::IoError)?;

        serde_json::to_writer_pretty(&mut file, &source_project)
            .map_err(ProjectSaveError::JsonError)?;

        Ok(())
    }

    /// Checks if there are any compatibility issues with this project file and
    /// warns the user if there are any.
    pub fn check_compatibility(&self) {
        let file_name = self.file_location
            .file_name().unwrap()
            .to_str().expect("Project file path was not valid Unicode!");

        if file_name == COMPAT_PROJECT_FILENAME {
            warn!("Rojo's default project file name changed in 0.5.0-alpha3.");
            warn!("Support for the old project file name will be dropped before 0.5.0 releases.");
            warn!("Your project file is named {}", COMPAT_PROJECT_FILENAME);
            warn!("Rename your project file to {}", PROJECT_FILENAME);
        } else if !file_name.ends_with(".project.json") {
            warn!("Starting in Rojo 0.5.0-alpha3, it's recommended to give all project files the");
            warn!(".project.json extension. This helps Rojo differentiate project files from");
            warn!("other JSON files!");
        }
    }

    fn to_source_project(&self) -> SourceProject {
        SourceProject {
            name: self.name.clone(),
            tree: self.tree.to_source_node(&self.file_location),
            serve_port: self.serve_port,
            serve_place_ids: self.serve_place_ids.clone(),
        }
    }
}