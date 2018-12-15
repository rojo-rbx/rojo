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
    assert_eq!(real.get_items(), &expected.items);
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
    let (root, imfs, expected_imfs, _) = base_tree()?;

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

    imfs.path_created(&add_one_path)?;
    imfs.path_created(&add_two_path)?;

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

    println!("{}", serde_json::to_string(imfs.get_items()).unwrap());
    println!("---------------------------------------------------------------");
    println!("{}", serde_json::to_string(&expected_imfs.items).unwrap());

    check_expected(&imfs, &expected_imfs);

    Ok(())
}