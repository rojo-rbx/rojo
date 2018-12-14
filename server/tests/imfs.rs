use std::{
    collections::{HashMap, HashSet},
    io,
    fs,
    path::PathBuf,
};

use tempfile::tempdir;

use librojo::{
    imfs::{Imfs, ImfsItem, ImfsFile, ImfsDirectory},
};

struct ExpectedImfs {
    roots: HashSet<PathBuf>,
    items: HashMap<PathBuf, ImfsItem>,
}

fn check_expected(real: &Imfs, expected: &ExpectedImfs) {
    assert_eq!(real.get_roots(), &expected.roots);
    assert_eq!(real.get_items(), &expected.items);
}

fn base_tree() -> io::Result<(Imfs, ExpectedImfs)> {
    let root = tempdir()?;

    let foo_path = root.path().join("foo");
    let bar_path = root.path().join("bar.txt");
    let baz_path = foo_path.join("baz.txt");

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

    Ok((imfs, expected_imfs))
}

#[test]
fn initial_read() -> io::Result<()> {
    let (imfs, expected_imfs) = base_tree()?;

    check_expected(&imfs, &expected_imfs);

    Ok(())
}