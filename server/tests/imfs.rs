use std::{
    collections::{HashMap, HashSet},
    io,
    fs,
    path::PathBuf,
};

use tempfile::{TempDir, tempdir};

use librojo::{
    imfs::{Imfs, ImfsItem, ImfsFile, ImfsDirectory},
};

#[allow(unused)]
enum FsEvent {
    Created(PathBuf),
    Updated(PathBuf),
    Removed(PathBuf),
    Moved(PathBuf, PathBuf),
}

fn send_events(imfs: &mut Imfs, events: &[FsEvent]) -> io::Result<()> {
    for event in events {
        match event {
            FsEvent::Created(path) => imfs.path_created(path)?,
            FsEvent::Updated(path) => imfs.path_updated(path)?,
            FsEvent::Removed(path) => imfs.path_removed(path)?,
            FsEvent::Moved(from, to) => imfs.path_moved(from, to)?,
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct ExpectedImfs {
    roots: HashSet<PathBuf>,
    items: HashMap<PathBuf, ImfsItem>,
}

struct TestResources {
    foo_path: PathBuf,
    bar_path: PathBuf,
    baz_path: PathBuf,
}

fn check_expected(real: &Imfs, expected: &ExpectedImfs) {
    assert_eq!(real.get_roots(), &expected.roots);

    let real_items = real.get_items();
    if real_items != &expected.items {
        let real_str = serde_json::to_string(real_items).unwrap();
        let expected_str = serde_json::to_string(&expected.items).unwrap();

        panic!("Items differed!\nReal:\n{}\nExpected:\n{}\n", real_str, expected_str);
    }
}

fn base_tree() -> io::Result<(TempDir, Imfs, ExpectedImfs, TestResources)> {
    let root = tempdir()?;

    let foo_path = root.path().join("foo");
    let bar_path = root.path().join("bar.txt");
    let baz_path = foo_path.join("baz.txt");

    let resources = TestResources {
        foo_path: foo_path.clone(),
        bar_path: bar_path.clone(),
        baz_path: baz_path.clone(),
    };

    fs::create_dir(&foo_path)?;
    fs::write(&bar_path, b"bar")?;
    fs::write(&baz_path, b"baz")?;

    let mut imfs = Imfs::new();
    imfs.add_root(root.path())?;

    let mut expected_roots = HashSet::new();
    expected_roots.insert(root.path().to_path_buf());

    let root_item = {
        let mut children = HashSet::new();
        children.insert(foo_path.clone());
        children.insert(bar_path.clone());

        ImfsItem::Directory(ImfsDirectory {
            path: root.path().to_path_buf(),
            children,
        })
    };

    let foo_item = {
        let mut children = HashSet::new();
        children.insert(baz_path.clone());

        ImfsItem::Directory(ImfsDirectory {
            path: foo_path.clone(),
            children,
        })
    };

    let bar_item = ImfsItem::File(ImfsFile {
        path: bar_path.clone(),
        contents: b"bar".to_vec(),
    });

    let baz_item = ImfsItem::File(ImfsFile {
        path: baz_path.clone(),
        contents: b"baz".to_vec(),
    });

    let mut expected_items = HashMap::new();
    expected_items.insert(root.path().to_path_buf(), root_item);
    expected_items.insert(foo_path.clone(), foo_item);
    expected_items.insert(bar_path.clone(), bar_item);
    expected_items.insert(baz_path.clone(), baz_item);

    let expected_imfs = ExpectedImfs {
        roots: expected_roots,
        items: expected_items,
    };

    Ok((root, imfs, expected_imfs, resources))
}

#[test]
fn initial_read() -> io::Result<()> {
    let (_root, imfs, expected_imfs, _resources) = base_tree()?;

    check_expected(&imfs, &expected_imfs);

    Ok(())
}

#[test]
fn adding_files() -> io::Result<()> {
    let (root, mut imfs, mut expected_imfs, resources) = base_tree()?;

    check_expected(&imfs, &expected_imfs);

    let add_one_path = root.path().join("add_one.txt");
    let add_two_path = resources.foo_path.join("add_two.txt");

    fs::write(&add_one_path, b"add_one")?;
    fs::write(&add_two_path, b"add_two")?;

    match expected_imfs.items.get_mut(root.path()) {
        Some(ImfsItem::Directory(directory)) => {
            directory.children.insert(add_one_path.clone());
        },
        _ => unreachable!(),
    }

    match expected_imfs.items.get_mut(&resources.foo_path) {
        Some(ImfsItem::Directory(directory)) => {
            directory.children.insert(add_two_path.clone());
        },
        _ => unreachable!(),
    }

    expected_imfs.items.insert(add_one_path.clone(), ImfsItem::File(ImfsFile {
        path: add_one_path.clone(),
        contents: b"add_one".to_vec(),
    }));

    expected_imfs.items.insert(add_two_path.clone(), ImfsItem::File(ImfsFile {
        path: add_two_path.clone(),
        contents: b"add_two".to_vec(),
    }));

    imfs.path_created(&add_one_path)?;
    imfs.path_created(&add_two_path)?;

    check_expected(&imfs, &expected_imfs);

    Ok(())
}

#[test]
fn adding_folder() -> io::Result<()> {
    let (root, imfs, mut expected_imfs, _resources) = base_tree()?;

    check_expected(&imfs, &expected_imfs);

    let folder_path = root.path().join("folder");
    let file1_path = folder_path.join("file1.txt");
    let file2_path = folder_path.join("file2.txt");

    fs::create_dir(&folder_path)?;
    fs::write(&file1_path, b"file1")?;
    fs::write(&file2_path, b"file2")?;

    match expected_imfs.items.get_mut(root.path()) {
        Some(ImfsItem::Directory(directory)) => {
            directory.children.insert(folder_path.clone());
        },
        _ => unreachable!(),
    }

    let folder_item = {
        let mut children = HashSet::new();
        children.insert(file1_path.clone());
        children.insert(file2_path.clone());

        ImfsItem::Directory(ImfsDirectory {
            path: folder_path.clone(),
            children,
        })
    };

    expected_imfs.items.insert(folder_path.clone(), folder_item);

    let file1_item = ImfsItem::File(ImfsFile {
        path: file1_path.clone(),
        contents: b"file1".to_vec(),
    });
    expected_imfs.items.insert(file1_path.clone(), file1_item);

    let file2_item = ImfsItem::File(ImfsFile {
        path: file2_path.clone(),
        contents: b"file2".to_vec(),
    });
    expected_imfs.items.insert(file2_path.clone(), file2_item);

    let possible_event_sequences = vec![
        vec![
            FsEvent::Created(folder_path.clone())
        ],
        vec![
            FsEvent::Created(folder_path.clone()),
            FsEvent::Created(file1_path.clone()),
            FsEvent::Created(file2_path.clone()),
        ],
    ];

    for events in &possible_event_sequences {
        let mut imfs = imfs.clone();

        send_events(&mut imfs, events)?;
        check_expected(&imfs, &expected_imfs);
    }

    Ok(())
}

#[test]
fn removing_file() -> io::Result<()> {
    let (root, mut imfs, mut expected_imfs, resources) = base_tree()?;

    check_expected(&imfs, &expected_imfs);

    fs::remove_file(&resources.bar_path)?;

    imfs.path_removed(&resources.bar_path)?;

    match expected_imfs.items.get_mut(root.path()) {
        Some(ImfsItem::Directory(directory)) => {
            directory.children.remove(&resources.bar_path);
        },
        _ => unreachable!(),
    }

    expected_imfs.items.remove(&resources.bar_path);

    check_expected(&imfs, &expected_imfs);

    Ok(())
}

#[test]
fn removing_folder() -> io::Result<()> {
    let (root, imfs, mut expected_imfs, resources) = base_tree()?;

    check_expected(&imfs, &expected_imfs);

    fs::remove_dir_all(&resources.foo_path)?;

    match expected_imfs.items.get_mut(root.path()) {
        Some(ImfsItem::Directory(directory)) => {
            directory.children.remove(&resources.foo_path);
        },
        _ => unreachable!(),
    }

    expected_imfs.items.remove(&resources.foo_path);
    expected_imfs.items.remove(&resources.baz_path);

    let possible_event_sequences = vec![
        vec![
            FsEvent::Removed(resources.foo_path.clone()),
        ],
        vec![
            FsEvent::Removed(resources.baz_path.clone()),
            FsEvent::Removed(resources.foo_path.clone()),
        ],
    ];

    for events in &possible_event_sequences {
        let mut imfs = imfs.clone();

        send_events(&mut imfs, events)?;
        check_expected(&imfs, &expected_imfs);
    }

    Ok(())
}