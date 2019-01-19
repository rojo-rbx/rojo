use std::{
    collections::HashMap,
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Deserialize)]
struct InputFile {
    path: PathBuf,
    contents: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct Metadata {
    ignore_unknown_instances: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "Type")]
enum Property {
    #[serde(rename_all = "PascalCase")]
    String {
        value: String,
    },
}

#[derive(Debug, Serialize)]
struct Instance {
    name: String,
    class_name: String,
    properties: HashMap<String, Property>,
    children: Vec<()>,
    metadata: Metadata,
}

fn main() -> io::Result<()> {
    let file: InputFile = bincode::deserialize_from(io::stdin())
        .expect("Invalid input");

    let mut child = Command::new("moonc")
        .arg("--")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(&file.contents)?;
    }

    let result = child.wait_with_output()?;

    let script_name = file.path.file_stem().unwrap().to_string_lossy().into_owned();
    let compiled = String::from_utf8_lossy(&result.stdout).into_owned();

    let mut properties = HashMap::new();
    properties.insert("Source".to_owned(), Property::String {
        value: compiled,
    });

    let output = Instance {
        name: script_name,
        class_name: "ModuleScript".to_owned(),
        properties,
        children: Vec::new(),
        metadata: Metadata {
            ignore_unknown_instances: false,
        },
    };

    let response = serde_json::to_string(&output)
        .expect("Couldn't serialize to JSON");

    print!("{}", response);

    Ok(())
}