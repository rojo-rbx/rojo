use std::env::current_dir;
use std::path::{Component, Path, PathBuf};

/// Converts a path to a 'route', used as the paths in Rojo.
pub fn path_to_route<A, B>(root: A, value: B) -> Option<Vec<String>>
where
    A: AsRef<Path>,
    B: AsRef<Path>,
{
    let root = root.as_ref();
    let value = value.as_ref();

    let relative = match value.strip_prefix(root) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let result = relative
        .components()
        .map(|component| {
            component.as_os_str().to_string_lossy().into_owned()
        })
        .collect::<Vec<_>>();

    Some(result)
}

#[test]
fn test_path_to_route() {
    fn t(root: &Path, value: &Path, result: Option<Vec<String>>) {
        assert_eq!(path_to_route(root, value), result);
    }

    t(Path::new("/a/b/c"), Path::new("/a/b/c/d"), Some(vec!["d".to_string()]));
    t(Path::new("/a/b"), Path::new("a"), None);
    t(Path::new("C:\\foo"), Path::new("C:\\foo\\bar\\baz"), Some(vec!["bar".to_string(), "baz".to_string()]));
}

/// Turns the path into an absolute one, using the current working directory if
/// necessary.
pub fn canonicalish<T: AsRef<Path>>(value: T) -> PathBuf {
    let cwd = current_dir().unwrap();

    absoluteify(&cwd, value)
}

/// Converts the given path to be absolute if it isn't already using a given
/// root.
pub fn absoluteify<A, B>(root: A, value: B) -> PathBuf
where
    A: AsRef<Path>,
    B: AsRef<Path>,
{
    let root = root.as_ref();
    let value = value.as_ref();

    if value.is_absolute() {
        PathBuf::from(value)
    } else {
        root.join(value)
    }
}

/// Collapses any `.` values along with any `..` values not at the start of the
/// path.
pub fn collapse<T: AsRef<Path>>(value: T) -> PathBuf {
    let value = value.as_ref();

    let mut buffer = Vec::new();

    for component in value.components() {
        match component {
            Component::ParentDir => match buffer.pop() {
                Some(_) => {},
                None => buffer.push(component.as_os_str()),
            },
            Component::CurDir => {},
            _ => {
                buffer.push(component.as_os_str());
            },
        }
    }

    buffer.iter().fold(PathBuf::new(), |mut acc, &x| {
        acc.push(x);
        acc
    })
}

#[test]
fn test_collapse() {
    fn identity(buf: PathBuf) {
        assert_eq!(buf, collapse(&buf));
    }

    identity(PathBuf::from("C:\\foo\\bar"));
    identity(PathBuf::from("/a/b/c"));
    identity(PathBuf::from("a/b"));

    assert_eq!(collapse(PathBuf::from("a/b/..")), PathBuf::from("a"));
    assert_eq!(collapse(PathBuf::from("./a/b/c/..")), PathBuf::from("a/b"));
    assert_eq!(collapse(PathBuf::from("../a")), PathBuf::from("../a"));
}
