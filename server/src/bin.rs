#[macro_use] extern crate clap;
#[macro_use] extern crate log;

use std::{
    path::{Path, PathBuf},
    process,
    env,
};

use librojo::commands;

fn make_path_absolute(value: &Path) -> PathBuf {
    if value.is_absolute() {
        PathBuf::from(value)
    } else {
        let current_dir = env::current_dir().unwrap();
        current_dir.join(value)
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let matches = clap_app!(rojo =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: env!("CARGO_PKG_DESCRIPTION"))

        (@subcommand init =>
            (about: "Creates a new Rojo project")
            (@arg PATH: "Path to the place to create the project. Defaults to the current directory.")
        )

        (@subcommand serve =>
            (about: "Serves the project's files for use with the Rojo Studio plugin.")
            (@arg PROJECT: "Path to the project to serve. Defaults to the current directory.")
            (@arg port: --port +takes_value "The port to listen on. Defaults to 8000.")
        )

        (@subcommand build =>
            (about: "Generates a .model.json, rbxm, or rbxmx model from the project.")
            (@arg PROJECT: "Path to the project to serve. Defaults to the current directory.")
            (@arg output: --output +takes_value +required "Where to output the result.")
        )
    ).get_matches();

    match matches.subcommand() {
        ("init", sub_matches) => {
            let sub_matches = sub_matches.unwrap();
            let project_path = Path::new(sub_matches.value_of("PATH").unwrap_or("."));
            let full_path = make_path_absolute(project_path);

            librojo::commands::init(&full_path);
        },
        ("serve", sub_matches) => {
            let sub_matches = sub_matches.unwrap();

            let project_path = match sub_matches.value_of("PROJECT") {
                Some(v) => make_path_absolute(Path::new(v)),
                None => std::env::current_dir().unwrap(),
            };

            librojo::commands::serve(&project_path);
        },
        ("build", sub_matches) => {
            let sub_matches = sub_matches.unwrap();

            let fuzzy_project_path = match sub_matches.value_of("PROJECT") {
                Some(v) => make_path_absolute(Path::new(v)),
                None => std::env::current_dir().unwrap(),
            };

            let output_file = make_path_absolute(Path::new(sub_matches.value_of("output").unwrap()));

            let options = commands::BuildOptions {
                fuzzy_project_path,
                output_file,
            };

            commands::build(&options);
        },
        _ => {
            error!("Please specify a subcommand!");
            error!("Try 'rojo help' for information.");
            process::exit(1);
        },
    }
}