use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs, io,
    net::IpAddr,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{glob::Glob, resolution::UnresolvedValue};

static PROJECT_FILENAME: &str = "default.project.json";

/// Error type returned by any function that handles projects.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ProjectError(#[from] Error);

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },

    #[error("Error parsing Rojo project in path {}", .path.display())]
    Json {
        source: serde_json::Error,
        path: PathBuf,
    },
}

/// Contains all of the configuration for a Rojo-managed project.
///
/// Project files are stored in `.project.json` files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Project {
    /// The name of the top-level instance described by the project.
    pub name: String,

    /// The tree of instances described by this project. Projects always
    /// describe at least one instance.
    pub tree: ProjectNode,

    /// If specified, sets the default port that `rojo serve` should use when
    /// using this project for live sync.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_port: Option<u16>,

    /// If specified, contains the set of place IDs that this project is
    /// compatible with when doing live sync.
    ///
    /// This setting is intended to help prevent syncing a Rojo project into the
    /// wrong Roblox place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_place_ids: Option<HashSet<u64>>,

    /// If specified, sets the current place's place ID when connecting to the
    /// Rojo server from Roblox Studio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_id: Option<u64>,

    /// If specified, sets the current place's game ID when connecting to the
    /// Rojo server from Roblox Studio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_id: Option<u64>,

    /// If specified, this address will be used in place of the default address
    /// As long as --address is unprovided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_address: Option<IpAddr>,

    /// A list of globs, relative to the folder the project file is in, that
    /// match files that should be excluded if Rojo encounters them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glob_ignore_paths: Vec<Glob>,

    /// The path to the file that this project came from. Relative paths in the
    /// project should be considered relative to the parent of this field, also
    /// given by `Project::folder_location`.
    #[serde(skip)]
    pub file_location: PathBuf,
}

impl Project {
    /// Tells whether the given path describes a Rojo project.
    pub fn is_project_file(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".project.json"))
            .unwrap_or(false)
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
    ) -> Result<Self, ProjectError> {
        let mut project: Self =
            serde_json::from_slice(&contents).map_err(|source| Error::Json {
                source,
                path: project_file_location.to_owned(),
            })?;

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

    fn load_exact(project_file_location: &Path) -> Result<Self, Error> {
        let contents = fs::read_to_string(project_file_location)?;

        let mut project: Project =
            serde_json::from_str(&contents).map_err(|source| Error::Json {
                source,
                path: project_file_location.to_owned(),
            })?;

        project.file_location = project_file_location.to_path_buf();
        project.check_compatibility();

        Ok(project)
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
pub struct OptionalPathNode {
    #[serde(serialize_with = "crate::path_serializer::serialize_absolute")]
    pub optional: PathBuf,
}

impl OptionalPathNode {
    pub fn new(optional: PathBuf) -> Self {
        OptionalPathNode { optional }
    }
}

/// Describes a path that is either optional or required
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathNode {
    Required(#[serde(serialize_with = "crate::path_serializer::serialize_absolute")] PathBuf),
    Optional(OptionalPathNode),
}

impl PathNode {
    pub fn path(&self) -> &Path {
        match self {
            PathNode::Required(pathbuf) => &pathbuf,
            PathNode::Optional(OptionalPathNode { optional }) => &optional,
        }
    }
}

/// Describes an instance and its descendants in a project.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectNode {
    /// If set, defines the ClassName of the described instance.
    ///
    /// `$className` MUST be set if `$path` is not set.
    ///
    /// `$className` CANNOT be set if `$path` is set and the instance described
    /// by that path has a ClassName other than Folder.
    #[serde(rename = "$className", skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    /// Contains all of the children of the described instance.
    #[serde(flatten)]
    pub children: BTreeMap<String, ProjectNode>,

    /// The properties that will be assigned to the resulting instance.
    ///
    // TODO: Is this legal to set if $path is set?
    #[serde(
        rename = "$properties",
        default,
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub properties: HashMap<String, UnresolvedValue>,

    #[serde(
        rename = "$attributes",
        default,
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub attributes: HashMap<String, UnresolvedValue>,

    /// Defines the behavior when Rojo encounters unknown instances in Roblox
    /// Studio during live sync. `$ignoreUnknownInstances` should be considered
    /// a large hammer and used with care.
    ///
    /// If set to `true`, those instances will be left alone. This may cause
    /// issues when files that turn into instances are removed while Rojo is not
    /// running.
    ///
    /// If set to `false`, Rojo will destroy any instances it does not
    /// recognize.
    ///
    /// If unset, its default value depends on other settings:
    /// - If `$path` is not set, defaults to `true`
    /// - If `$path` is set, defaults to `false`
    #[serde(
        rename = "$ignoreUnknownInstances",
        skip_serializing_if = "Option::is_none"
    )]
    pub ignore_unknown_instances: Option<bool>,

    /// Defines that this instance should come from the given file path. This
    /// path can point to any file type supported by Rojo, including Lua files
    /// (`.lua`), Roblox models (`.rbxm`, `.rbxmx`), and localization table
    /// spreadsheets (`.csv`).
    #[serde(rename = "$path", skip_serializing_if = "Option::is_none")]
    pub path: Option<PathNode>,
}

impl ProjectNode {
    fn validate_reserved_names(&self) {
        for (name, child) in &self.children {
            if name.starts_with('$') {
                log::warn!(
                    "Keys starting with '$' are reserved by Rojo to ensure forward compatibility."
                );
                log::warn!(
                    "This project uses the key '{}', which should be renamed.",
                    name
                );
            }

            child.validate_reserved_names();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn path_node_required() {
        let path_node: PathNode = serde_json::from_str(r#""src""#).unwrap();
        assert_eq!(path_node, PathNode::Required(PathBuf::from("src")));
    }

    #[test]
    fn path_node_optional() {
        let path_node: PathNode = serde_json::from_str(r#"{ "optional": "src" }"#).unwrap();
        assert_eq!(
            path_node,
            PathNode::Optional(OptionalPathNode::new(PathBuf::from("src")))
        );
    }

    #[test]
    fn project_node_required() {
        let project_node: ProjectNode = serde_json::from_str(
            r#"{
                "$path": "src"
            }"#,
        )
        .unwrap();

        assert_eq!(
            project_node.path,
            Some(PathNode::Required(PathBuf::from("src")))
        );
    }

    #[test]
    fn project_node_optional() {
        let project_node: ProjectNode = serde_json::from_str(
            r#"{
                "$path": { "optional": "src" }
            }"#,
        )
        .unwrap();

        assert_eq!(
            project_node.path,
            Some(PathNode::Optional(OptionalPathNode::new(PathBuf::from(
                "src"
            ))))
        );
    }

    #[test]
    fn project_node_none() {
        let project_node: ProjectNode = serde_json::from_str(
            r#"{
                "$className": "Folder"
            }"#,
        )
        .unwrap();

        assert_eq!(project_node.path, None);
    }

    #[test]
    fn project_node_optional_serialize_absolute() {
        let project_node: ProjectNode = serde_json::from_str(
            r#"{
                "$path": { "optional": "..\\src" }
            }"#,
        )
        .unwrap();

        let serialized = serde_json::to_string(&project_node).unwrap();
        assert_eq!(serialized, r#"{"$path":{"optional":"../src"}}"#);
    }

    #[test]
    fn project_node_optional_serialize_absolute_no_change() {
        let project_node: ProjectNode = serde_json::from_str(
            r#"{
                "$path": { "optional": "../src" }
            }"#,
        )
        .unwrap();

        let serialized = serde_json::to_string(&project_node).unwrap();
        assert_eq!(serialized, r#"{"$path":{"optional":"../src"}}"#);
    }

    #[test]
    fn project_node_optional_serialize_optional() {
        let project_node: ProjectNode = serde_json::from_str(
            r#"{
                "$path": "..\\src"
            }"#,
        )
        .unwrap();

        let serialized = serde_json::to_string(&project_node).unwrap();
        assert_eq!(serialized, r#"{"$path":"../src"}"#);
    }
}
