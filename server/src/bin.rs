#[macro_use] extern crate clap;

extern crate rojo_core;

use std::path::{Path, PathBuf};
use std::process;

use rojo_core::pathext::canonicalish;

fn main() {
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

        (@subcommand pack =>
            (about: "Packs the project into a GUI installer bundle. NOT YET IMPLEMENTED!")
            (@arg PROJECT: "Path to the project to pack. Defaults to the current directory.")
        )

        (@arg verbose: --verbose "Enable extended logging.")
    ).get_matches();

    let verbose = match matches.occurrences_of("verbose") {
        0 => false,
        _ => true,
    };

    match matches.subcommand() {
        ("init", sub_matches) => {
            let sub_matches = sub_matches.unwrap();
            let project_path = Path::new(sub_matches.value_of("PATH").unwrap_or("."));
            let full_path = canonicalish(project_path);

            rojo_core::commands::init(&full_path);
        },
        ("serve", sub_matches) => {
            let sub_matches = sub_matches.unwrap();

            let project_path = match sub_matches.value_of("PROJECT") {
                Some(v) => canonicalish(PathBuf::from(v)),
                None => std::env::current_dir().unwrap(),
            };

            let port = {
                match sub_matches.value_of("port") {
                    Some(source) => match source.parse::<u64>() {
                        Ok(value) => Some(value),
                        Err(_) => {
                            eprintln!("Invalid port '{}'", source);
                            process::exit(1);
                        },
                    },
                    None => None,
                }
            };

            rojo_core::commands::serve(&project_path, verbose, port);
        },
        ("pack", _) => {
            eprintln!("'rojo pack' is not yet implemented!");
            process::exit(1);
        },
        _ => {
            eprintln!("Please specify a subcommand!");
            eprintln!("Try 'rojo help' for information.");
            process::exit(1);
        },
    }
}
