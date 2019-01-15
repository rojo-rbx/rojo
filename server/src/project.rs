use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use failure::Fail;
use maplit::hashmap;
use rbx_tree::RbxValue;
use serde_derive::{Serialize, Deserialize};

pub static PROJECT_FILENAME: &'static str = "roblox-project.json";

// Methods used for Serde's default value system, which doesn't support using
// value literals directly, only functions that return values.
const fn yeah() -> bool {
    true
}

const fn is_true(value: &bool) -> bool {
    *value
}

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
#[serde(untagged)]
enum SourceProjectNode {
    Instance {
        #[serde(rename = "$className")]
        class_name: String,

        #[serde(rename = "$properties", default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
        properties: HashMap<String, RbxValue>,

        #[serde(rename = "$ignoreUnknownInstances", default = "yeah", skip_serializing_if = "is_true")]
        ignore_unknown_instances: bool,

        #[serde(flatten)]
        children: HashMap<String, SourceProjectNode>,
    },
    SyncPoint {
        #[serde(rename = "$path")]
        path: String,
    }
}

impl SourceProjectNode {
    /// Consumes the SourceProjectNode and turns it into a ProjectNode.
    pub fn into_project_node(self, project_file_location: &Path) -> ProjectNode {
        match self {
            SourceProjectNode::Instance { class_name, mut children, properties, ignore_unknown_instances } => {
                let mut new_children = HashMap::new();

                for (node_name, node) in children.drain() {
                    new_children.insert(node_name, node.into_project_node(project_file_location));
                }

                ProjectNode::Instance(InstanceProjectNode {
                    class_name,
                    children: new_children,
                    properties,
                    metadata: InstanceProjectNodeMetadata {
                        ignore_unknown_instances,
                    },
                })
            },
            SourceProjectNode::SyncPoint { path: source_path } => {
                let path = if Path::new(&source_path).is_absolute() {
                    PathBuf::from(source_path)
                } else {
                    let project_folder_location = project_file_location.parent().unwrap();
                    project_folder_location.join(source_path)
                };

                ProjectNode::SyncPoint(SyncPointProjectNode {
                    path,
                })
            },
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceProjectNodeMetadata {
    pub ignore_unknown_instances: bool,
}

impl Default for InstanceProjectNodeMetadata {
    fn default() -> InstanceProjectNodeMetadata {
        InstanceProjectNodeMetadata {
            ignore_unknown_instances: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProjectNode {
    Instance(InstanceProjectNode),
    SyncPoint(SyncPointProjectNode),
}

impl ProjectNode {
    fn to_source_node(&self, project_file_location: &Path) -> SourceProjectNode {
        match self {
            ProjectNode::Instance(node) => {
                let mut children = HashMap::new();

                for (key, child) in &node.children {
                    children.insert(key.clone(), child.to_source_node(project_file_location));
                }

                SourceProjectNode::Instance {
                    class_name: node.class_name.clone(),
                    children,
                    properties: node.properties.clone(),
                    ignore_unknown_instances: node.metadata.ignore_unknown_instances,
                }
            },
            ProjectNode::SyncPoint(sync_node) => {
                let project_folder_location = project_file_location.parent().unwrap();

                let friendly_path = match sync_node.path.strip_prefix(project_folder_location) {
                    Ok(stripped) => stripped.to_str().unwrap().replace("\\", "/"),
                    Err(_) => format!("{}", sync_node.path.display()),
                };

                SourceProjectNode::SyncPoint {
                    path: friendly_path,
                }
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceProjectNode {
    pub class_name: String,
    pub children: HashMap<String, ProjectNode>,
    pub properties: HashMap<String, RbxValue>,
    pub metadata: InstanceProjectNodeMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPointProjectNode {
    pub path: PathBuf,
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

        let tree = ProjectNode::Instance(InstanceProjectNode {
            class_name: "DataModel".to_string(),
            children: hashmap! {
                String::from("ReplicatedStorage") => ProjectNode::Instance(InstanceProjectNode {
                    class_name: String::from("ReplicatedStorage"),
                    children: hashmap! {
                        String::from("Source") => ProjectNode::SyncPoint(SyncPointProjectNode {
                            path: project_folder_path.join("src"),
                        }),
                    },
                    properties: HashMap::new(),
                    metadata: Default::default(),
                }),
                String::from("HttpService") => ProjectNode::Instance(InstanceProjectNode {
                    class_name: String::from("HttpService"),
                    children: HashMap::new(),
                    properties: hashmap! {
                        String::from("HttpEnabled") => RbxValue::Bool {
                            value: true,
                        },
                    },
                    metadata: Default::default(),
                }),
            },
            properties: HashMap::new(),
            metadata: Default::default(),
        });

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

        let tree = ProjectNode::SyncPoint(SyncPointProjectNode {
            path: project_folder_path.join("src"),
        });

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

            if let Ok(with_file_metadata) = fs::metadata(&with_file) {
                if with_file_metadata.is_file() {
                    return Some(with_file);
                } else {
                    return None;
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

    fn to_source_project(&self) -> SourceProject {
        SourceProject {
            name: self.name.clone(),
            tree: self.tree.to_source_node(&self.file_location),
            serve_port: self.serve_port,
            serve_place_ids: self.serve_place_ids.clone(),
        }
    }
}