use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{glob::Glob, path_serializer, project::ProjectNode};

/// Rojo-specific metadata that can be associated with an instance or a snapshot
/// of an instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceMetadata {
    /// Whether instances not present in the source should be ignored when
    /// live-syncing. This is useful when there are instances that Rojo does not
    /// manage.
    pub ignore_unknown_instances: bool,

    /// If a change occurs to this instance, the instigating source is what
    /// should be run through the snapshot functions to regenerate it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instigating_source: Option<InstigatingSource>,

    /// The paths that, when changed, could cause the function that generated
    /// this snapshot to generate a different snapshot. Paths should be included
    /// even if they don't exist, since the presence of a file can change the
    /// outcome of a snapshot function.
    ///
    /// For example, a file named foo.lua might have these relevant paths:
    /// - foo.lua
    /// - foo.meta.json (even if this file doesn't exist!)
    ///
    /// A directory named bar/ might have these:
    /// - bar/
    /// - bar/init.meta.json
    /// - bar/init.lua
    /// - bar/init.server.lua
    /// - bar/init.client.lua
    /// - bar/default.project.json
    ///
    /// This path is used to make sure that file changes update all instances
    /// that may need updates.
    // TODO: Change this to be a SmallVec for performance in common cases?
    #[serde(serialize_with = "path_serializer::serialize_vec_absolute")]
    pub relevant_paths: Vec<PathBuf>,

    /// Contains information about this instance that should persist between
    /// snapshot invocations and is generally inherited.
    ///
    /// If an instance has a piece of context attached to it, then the next time
    /// that instance's instigating source is snapshotted directly, the same
    /// context will be passed into it.
    pub context: InstanceContext,
}

impl InstanceMetadata {
    pub fn new() -> Self {
        Self {
            ignore_unknown_instances: false,
            instigating_source: None,
            relevant_paths: Vec::new(),
            context: InstanceContext::default(),
        }
    }

    pub fn ignore_unknown_instances(self, ignore_unknown_instances: bool) -> Self {
        Self {
            ignore_unknown_instances,
            ..self
        }
    }

    pub fn instigating_source(self, instigating_source: impl Into<InstigatingSource>) -> Self {
        Self {
            instigating_source: Some(instigating_source.into()),
            ..self
        }
    }

    pub fn relevant_paths(self, relevant_paths: Vec<PathBuf>) -> Self {
        Self {
            relevant_paths,
            ..self
        }
    }

    pub fn context(self, context: &InstanceContext) -> Self {
        Self {
            context: context.clone(),
            ..self
        }
    }
}

impl Default for InstanceMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceContext {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path_ignore_rules: Arc<Vec<PathIgnoreRule>>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_override_rules: Arc<Vec<TypeOverrideRule>>,
}

impl InstanceContext {
    /// Extend the list of ignore rules in the context with the given new rules.
    pub fn add_path_ignore_rules<I>(&mut self, new_rules: I)
    where
        I: IntoIterator<Item = PathIgnoreRule>,
        I::IntoIter: ExactSizeIterator,
    {
        let new_rules = new_rules.into_iter();

        // If the iterator is empty, we can skip cloning our list of ignore
        // rules and appending to it.
        if new_rules.len() == 0 {
            return;
        }

        let rules = Arc::make_mut(&mut self.path_ignore_rules);
        rules.extend(new_rules);
    }

    /// Extend the list of type override rules in the context with the given new rules.
    pub fn add_type_override_rules<I>(&mut self, new_rules: I)
    where
        I: IntoIterator<Item = TypeOverrideRule>,
        I::IntoIter: ExactSizeIterator,
    {
        let new_rules = new_rules.into_iter();

        // If the iterator is empty, we can skip cloning our list of ignore
        // rules and appending to it.
        if new_rules.len() == 0 {
            return;
        }

        let rules = Arc::make_mut(&mut self.type_override_rules);
        rules.extend(new_rules);
    }

    pub fn get_type_override(&self, path: &Path) -> Option<RojoType> {
        for rule in self.type_override_rules.iter() {
            if rule.applies_to(path) {
                return Some(RojoType::from_str(&rule.type_name));
            }
        }

        None
    }
}

impl Default for InstanceContext {
    fn default() -> Self {
        InstanceContext {
            path_ignore_rules: Arc::new(Vec::new()),
            type_override_rules: Arc::new(Vec::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathIgnoreRule {
    /// The path that this glob is relative to. Since ignore globs are defined
    /// in project files, this will generally be the folder containing the
    /// project file that defined this glob.
    #[serde(serialize_with = "path_serializer::serialize_absolute")]
    pub base_path: PathBuf,

    /// The actual glob that can be matched against the input path.
    pub glob: Glob,
}

impl PathIgnoreRule {
    pub fn passes<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();

        match path.strip_prefix(&self.base_path) {
            Ok(suffix) => !self.glob.is_match(suffix),
            Err(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RojoType {
    Plain,
    LuauModule,
    LuauServer,
    LuauClient,
    Json,
    Csv,

    Project,
    Rbxm,
    Rbxmx,
    JsonModel,

    Other(String),
}

impl RojoType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "rojo/plaintext" => Self::Plain,
            "rojo/luau" => Self::LuauModule,
            "rojo/luauserver" => Self::LuauServer,
            "rojo/luauclient" => Self::LuauClient,
            "rojo/json" => Self::Json,
            "rojo/csv" => Self::Csv,

            "rojo/project" => Self::Project,
            "rojo/rbxm" => Self::Rbxm,
            "rojo/rbxmx" => Self::Rbxmx,
            "rojo/jsonmodel" => Self::JsonModel,

            _ => Self::Other(s.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeOverrideRule {
    /// The glob to match files against for this type override
    pub pattern: Glob,

    /// The type of file this match should be treated as
    pub type_name: String,

    /// The path that this glob is relative to. Since ignore globs are defined
    /// in project files, this will generally be the folder containing the
    /// project file that defined this glob.
    #[serde(serialize_with = "path_serializer::serialize_absolute")]
    pub base_path: PathBuf,
}

impl TypeOverrideRule {
    pub fn applies_to<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();

        match path.strip_prefix(&self.base_path) {
            Ok(suffix) => self.pattern.is_match(suffix),
            Err(_) => false,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum InstigatingSource {
    Path(#[serde(serialize_with = "path_serializer::serialize_absolute")] PathBuf),
    ProjectNode(
        #[serde(serialize_with = "path_serializer::serialize_absolute")] PathBuf,
        String,
        ProjectNode,
        Option<String>,
    ),
}

impl fmt::Debug for InstigatingSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstigatingSource::Path(path) => write!(formatter, "Path({})", path.display()),
            InstigatingSource::ProjectNode(path, name, node, parent_class) => write!(
                formatter,
                "ProjectNode({}: {:?}) from path {} and parent class {:?}",
                name,
                node,
                path.display(),
                parent_class,
            ),
        }
    }
}

impl From<PathBuf> for InstigatingSource {
    fn from(path: PathBuf) -> Self {
        InstigatingSource::Path(path)
    }
}

impl From<&Path> for InstigatingSource {
    fn from(path: &Path) -> Self {
        InstigatingSource::Path(path.to_path_buf())
    }
}
