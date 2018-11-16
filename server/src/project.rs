use std::{
    collections::HashMap,
    fmt,
    fs,
    io,
    path::{Path, PathBuf},
};
use serde_json;

pub static PROJECT_FILENAME: &'static str = "roblox-project.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum SourceProjectNode {
    Regular {
        #[serde(rename = "$className")]
        class_name: String,

        // #[serde(rename = "$ignoreUnknown", default = "false")]
        // ignore_unknown: bool,

        #[serde(flatten)]
        children: HashMap<String, SourceProjectNode>,
    },
    SyncPoint {
        #[serde(rename = "$path")]
        path: String,
    }
}

impl SourceProjectNode {
    pub fn into_project_node(self, project_file_location: &Path) -> ProjectNode {
        match self {
            SourceProjectNode::Regular { class_name, mut children } => {
                let mut new_children = HashMap::new();

                for (node_name, node) in children.drain() {
                    new_children.insert(node_name, node.into_project_node(project_file_location));
                }

                ProjectNode::Regular {
                    class_name,
                    children: new_children,
                }
            },
            SourceProjectNode::SyncPoint { path: source_path } => {
                let path = if Path::new(&source_path).is_absolute() {
                    PathBuf::from(source_path)
                } else {
                    let project_folder_location = project_file_location.parent().unwrap();
                    project_folder_location.join(source_path)
                };

                ProjectNode::SyncPoint {
                    path,
                }
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceProject {
    name: String,
    tree: SourceProjectNode,
}

impl SourceProject {
    pub fn into_project(self, project_file_location: &Path) -> Project {
        let tree = self.tree.into_project_node(project_file_location);

        Project {
            name: self.name,
            tree,
            file_location: PathBuf::from(project_file_location),
        }
    }
}

#[derive(Debug)]
pub enum ProjectLoadExactError {
    IoError(io::Error),
    JsonError(serde_json::Error),
}

impl fmt::Display for ProjectLoadExactError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProjectLoadExactError::IoError(inner) => write!(output, "{}", inner),
            ProjectLoadExactError::JsonError(inner) => write!(output, "{}", inner),
        }
    }
}

#[derive(Debug)]
pub enum ProjectInitError {}

impl fmt::Display for ProjectInitError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        write!(output, "ProjectInitError")
    }
}

#[derive(Debug)]
pub enum ProjectLoadFuzzyError {
    NotFound,
    IoError(io::Error),
    JsonError(serde_json::Error),
}

impl From<ProjectLoadExactError> for ProjectLoadFuzzyError {
    fn from(error: ProjectLoadExactError) -> ProjectLoadFuzzyError {
        match error {
            ProjectLoadExactError::IoError(inner) => ProjectLoadFuzzyError::IoError(inner),
            ProjectLoadExactError::JsonError(inner) => ProjectLoadFuzzyError::JsonError(inner),
        }
    }
}

impl fmt::Display for ProjectLoadFuzzyError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProjectLoadFuzzyError::NotFound => write!(output, "Project not found."),
            ProjectLoadFuzzyError::IoError(inner) => write!(output, "{}", inner),
            ProjectLoadFuzzyError::JsonError(inner) => write!(output, "{}", inner),
        }
    }
}

#[derive(Debug)]
pub enum ProjectSaveError {}

impl fmt::Display for ProjectSaveError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        write!(output, "ProjectSaveError")
    }
}

#[derive(Debug)]
pub enum ProjectNode {
    Regular {
        class_name: String,
        children: HashMap<String, ProjectNode>,

        // ignore_unknown: bool,
    },
    SyncPoint {
        path: PathBuf,
    },
}

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub tree: ProjectNode,
    pub file_location: PathBuf,
}

impl Project {
    pub fn init(_project_folder_location: &Path) -> Result<(), ProjectInitError> {
        unimplemented!();
    }

    pub fn locate(start_location: &Path) -> Option<PathBuf> {
        // TODO: Check for specific error kinds, convert 'not found' to Result.
        let location_metadata = fs::metadata(start_location).ok()?;

        // If this is a file, we should assume it's the config we want
        if location_metadata.is_file() {
            return Some(start_location.to_path_buf());
        } else if location_metadata.is_dir() {
            let with_file = start_location.join(PROJECT_FILENAME);

            match fs::metadata(&with_file) {
                Ok(with_file_metadata) => {
                    if with_file_metadata.is_file() {
                        return Some(with_file);
                    } else {
                        return None;
                    }
                },
                Err(_) => {},
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
        let _source_project = self.to_source_project();

        unimplemented!();
    }

    fn to_source_project(&self) -> SourceProject {
        unimplemented!();
    }
}