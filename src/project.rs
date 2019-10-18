use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use failure::Fail;
use log::warn;
use rbx_dom_weak::{RbxValue, UnresolvedRbxValue};
use serde::{Deserialize, Serialize, Serializer};

static DEFAULT_PLACE: &str = include_str!("../assets/place.project.json");

pub static PROJECT_FILENAME: &str = "default.project.json";
pub static COMPAT_PROJECT_FILENAME: &str = "roblox-project.json";

/// SourceProject is the format that users author projects on-disk. Since we
/// want to do things like transforming paths to be absolute before handing them
/// off to the rest of Rojo, we use this intermediate struct.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SourceProject {
    name: String,
    tree: SourceProjectNode,

    #[serde(skip_serializing_if = "Option::is_none")]
    serve_port: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    serve_place_ids: Option<HashSet<u64>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(not(feature = "user-plugins"), serde(skip_deserializing))]
    plugins: Vec<String>,
}

impl SourceProject {
    /// Consumes the SourceProject and yields a Project, ready for prime-time.
    pub fn into_project(self, project_file_location: &Path) -> Project {
        let tree = self.tree.into_project_node(project_file_location);

        let project_folder = project_file_location.parent().unwrap();
        let plugins = self
            .plugins
            .into_iter()
            .map(|path| project_folder.join(path))
            .collect();

        Project {
            name: self.name,
            tree,
            serve_port: self.serve_port,
            serve_place_ids: self.serve_place_ids,
            plugins,
            file_location: PathBuf::from(project_file_location),
        }
    }
}

/// An alternative serializer for `UnresolvedRbxValue` that uses the minimum
/// representation of the value.
///
/// For example, the default Serialize impl might give you:
///
/// ```json
/// {
///     "Type": "Bool",
///     "Value": true
/// }
/// ```
///
/// But in reality, users are expected to write just:
///
/// ```json
/// true
/// ```
///
/// This holds true for other values that might be ambiguous or just have more
/// complicated representations like enums.
fn serialize_unresolved_minimal<S>(
    unresolved: &UnresolvedRbxValue,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match unresolved {
        UnresolvedRbxValue::Ambiguous(_) => unresolved.serialize(serializer),
        UnresolvedRbxValue::Concrete(concrete) => match concrete {
            RbxValue::Bool { value } => value.serialize(serializer),
            RbxValue::CFrame { value } => value.serialize(serializer),
            RbxValue::Color3 { value } => value.serialize(serializer),
            RbxValue::Color3uint8 { value } => value.serialize(serializer),
            RbxValue::Content { value } => value.serialize(serializer),
            RbxValue::Float32 { value } => value.serialize(serializer),
            RbxValue::Int32 { value } => value.serialize(serializer),
            RbxValue::String { value } => value.serialize(serializer),
            RbxValue::UDim { value } => value.serialize(serializer),
            RbxValue::UDim2 { value } => value.serialize(serializer),
            RbxValue::Vector2 { value } => value.serialize(serializer),
            RbxValue::Vector2int16 { value } => value.serialize(serializer),
            RbxValue::Vector3 { value } => value.serialize(serializer),
            RbxValue::Vector3int16 { value } => value.serialize(serializer),
            _ => concrete.serialize(serializer),
        },
    }
}

/// A wrapper around serialize_unresolved_minimal that handles the HashMap case.
fn serialize_unresolved_map<S>(
    value: &HashMap<String, UnresolvedRbxValue>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeMap;

    #[derive(Serialize)]
    struct Minimal<'a>(
        #[serde(serialize_with = "serialize_unresolved_minimal")] &'a UnresolvedRbxValue,
    );

    let mut map = serializer.serialize_map(Some(value.len()))?;
    for (k, v) in value {
        map.serialize_key(k)?;
        map.serialize_value(&Minimal(v))?;
    }
    map.end()
}

/// Similar to SourceProject, the structure of nodes in the project tree is
/// slightly different on-disk than how we want to handle them in the rest of
/// Rojo.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceProjectNode {
    #[serde(rename = "$className", skip_serializing_if = "Option::is_none")]
    class_name: Option<String>,

    #[serde(
        rename = "$properties",
        default = "HashMap::new",
        skip_serializing_if = "HashMap::is_empty",
        serialize_with = "serialize_unresolved_map"
    )]
    properties: HashMap<String, UnresolvedRbxValue>,

    #[serde(
        rename = "$ignoreUnknownInstances",
        skip_serializing_if = "Option::is_none"
    )]
    ignore_unknown_instances: Option<bool>,

    #[serde(rename = "$path", skip_serializing_if = "Option::is_none")]
    path: Option<String>,

    #[serde(flatten)]
    children: BTreeMap<String, SourceProjectNode>,
}

impl SourceProjectNode {
    /// Consumes the SourceProjectNode and turns it into a ProjectNode.
    pub fn into_project_node(self, project_file_location: &Path) -> ProjectNode {
        let children = self
            .children
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value.clone().into_project_node(project_file_location),
                )
            })
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
    pub class_name: Option<String>,
    pub children: BTreeMap<String, ProjectNode>,
    pub properties: HashMap<String, UnresolvedRbxValue>,
    pub ignore_unknown_instances: Option<bool>,

    #[serde(serialize_with = "crate::path_serializer::serialize_option")]
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

    fn to_source_node(&self, project_file_location: &Path) -> SourceProjectNode {
        let children = self
            .children
            .iter()
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
    pub plugins: Vec<PathBuf>,
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

        let mut project = Project::load_from_str(DEFAULT_PLACE, &project_path)
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
            plugins: Vec::new(),
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

    fn locate(start_location: &Path) -> Option<PathBuf> {
        // TODO: Check for specific error kinds, convert 'not found' to Result.
        let location_metadata = fs::metadata(start_location).ok()?;

        // If this is a file, assume it's the config the user was looking for.
        if location_metadata.is_file() {
            if Project::is_project_file(start_location) {
                return Some(start_location.to_path_buf());
            } else {
                return None;
            }
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

    fn load_from_str(
        contents: &str,
        project_file_location: &Path,
    ) -> Result<Project, serde_json::Error> {
        let parsed: SourceProject = serde_json::from_str(&contents)?;

        Ok(parsed.into_project(project_file_location))
    }

    pub fn load_from_slice(
        contents: &[u8],
        project_file_location: &Path,
    ) -> Result<Project, serde_json::Error> {
        let parsed: SourceProject = serde_json::from_slice(&contents)?;

        Ok(parsed.into_project(project_file_location))
    }

    pub fn load_fuzzy(fuzzy_project_location: &Path) -> Result<Project, ProjectLoadError> {
        if let Some(project_path) = Self::locate(fuzzy_project_location) {
            Self::load_exact(&project_path)
        } else {
            Project::warn_if_4x_project_present(fuzzy_project_location);
            Err(ProjectLoadError::NotFound)
        }
    }

    pub fn load_exact(project_file_location: &Path) -> Result<Project, ProjectLoadError> {
        let contents =
            fs::read_to_string(project_file_location).map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => ProjectLoadError::NotFound,
                _ => ProjectLoadError::Io {
                    inner: error,
                    path: project_file_location.to_path_buf(),
                },
            })?;

        let parsed: SourceProject =
            serde_json::from_str(&contents).map_err(|error| ProjectLoadError::Json {
                inner: error,
                path: project_file_location.to_path_buf(),
            })?;

        let project = parsed.into_project(project_file_location);
        project.check_compatibility();

        Ok(project)
    }

    pub fn save(&self) -> Result<(), ProjectSaveError> {
        let source_project = self.to_source_project();
        let mut file = File::create(&self.file_location).map_err(ProjectSaveError::IoError)?;

        serde_json::to_writer_pretty(&mut file, &source_project)
            .map_err(ProjectSaveError::JsonError)?;

        Ok(())
    }

    /// Checks if there are any compatibility issues with this project file and
    /// warns the user if there are any.
    fn check_compatibility(&self) {
        let file_name = self
            .file_location
            .file_name()
            .expect("Project file path did not have a file name")
            .to_str()
            .expect("Project file path was not valid Unicode");

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

        self.tree.validate_reserved_names();
    }

    /// Issues a warning if no Rojo 0.5.x project is found, but there's a legacy
    /// 0.4.x project in the directory.
    fn warn_if_4x_project_present(folder: &Path) {
        let file_path = folder.join("rojo.json");

        if fs::metadata(file_path).is_ok() {
            warn!("No Rojo 0.5 project file was found, but a Rojo 0.4 project was.");
            warn!("Rojo 0.5.x uses 'default.project.json' files");
            warn!("Rojo 0.5.x uses 'rojo.json' files");
            warn!("");
            warn!("For help upgrading, see:");
            warn!("https://lpghatguy.github.io/rojo/guide/migrating-to-epiphany/");
        }
    }

    pub fn folder_location(&self) -> &Path {
        self.file_location.parent().unwrap()
    }

    fn to_source_project(&self) -> SourceProject {
        // TODO: Use path_serializer instead of transforming paths between
        // String and PathBuf?
        let plugins = self
            .plugins
            .iter()
            .map(|path| {
                path.strip_prefix(self.folder_location())
                    .unwrap()
                    .display()
                    .to_string()
            })
            .collect();

        SourceProject {
            name: self.name.clone(),
            tree: self.tree.to_source_node(&self.file_location),
            serve_port: self.serve_port,
            plugins,
            serve_place_ids: self.serve_place_ids.clone(),
        }
    }
}
