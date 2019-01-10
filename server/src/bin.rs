#[macro_use] extern crate log;

use std::{
    path::{Path, PathBuf},
    env,
    process,
};

use clap::clap_app;

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

    let mut app = clap_app!(Rojo =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: env!("CARGO_PKG_DESCRIPTION"))

        (@subcommand init =>
            (about: "Creates a new Rojo project")
            (@arg PATH: "Path to the place to create the project. Defaults to the current directory.")
            (@arg kind: --kind +takes_value "The kind of project to create, 'place' or 'model'. Defaults to place.")
        )

        (@subcommand serve =>
            (about: "Serves the project's files for use with the Rojo Studio plugin.")
            (@arg PROJECT: "Path to the project to serve. Defaults to the current directory.")
            (@arg port: --port +takes_value "The port to listen on. Defaults to 8000.")
        )

        (@subcommand build =>
            (about: "Generates an rbxmx model file from the project.")
            (@arg PROJECT: "Path to the project to serve. Defaults to the current directory.")
            (@arg output: --output -o +takes_value +required "Where to output the result.")
        )

        (@subcommand upload =>
            (about: "Generates a place file out of the project and uploads it to Roblox.")
            (@arg PROJECT: "Path to the project to upload. Defaults to the current directory.")
            (@arg cookie: --cookie +takes_value +required "Security cookie to authenticate with.")
            (@arg place_id: --place_id +takes_value +required "Place ID to upload to.")
        )
    );

    // `get_matches` consumes self for some reason.
    let matches = app.clone().get_matches();

    match matches.subcommand() {
        ("init", Some(sub_matches)) => {
            let fuzzy_project_path = make_path_absolute(Path::new(sub_matches.value_of("PATH").unwrap_or("")));
            let kind = sub_matches.value_of("kind");

            let options = commands::InitOptions {
                fuzzy_project_path,
                kind,
            };

            match commands::init(&options) {
                Ok(_) => {},
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                },
            }
        },
        ("serve", Some(sub_matches)) => {
            let fuzzy_project_path = match sub_matches.value_of("PROJECT") {
                Some(v) => make_path_absolute(Path::new(v)),
                None => std::env::current_dir().unwrap(),
            };

            let port = match sub_matches.value_of("port") {
                Some(v) => match v.parse::<u16>() {
                    Ok(port) => Some(port),
                    Err(_) => {
                        error!("Invalid port {}", v);
                        process::exit(1);
                    },
                },
                None => None,
            };

            let options = commands::ServeOptions {
                fuzzy_project_path,
                port,
            };

            match commands::serve(&options) {
                Ok(_) => {},
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                },
            }
        },
        ("build", Some(sub_matches)) => {
            let fuzzy_project_path = match sub_matches.value_of("PROJECT") {
                Some(v) => make_path_absolute(Path::new(v)),
                None => std::env::current_dir().unwrap(),
            };

            let output_file = make_path_absolute(Path::new(sub_matches.value_of("output").unwrap()));

            let options = commands::BuildOptions {
                fuzzy_project_path,
                output_file,
                output_kind: None, // TODO: Accept from argument
            };

            match commands::build(&options) {
                Ok(_) => {},
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                },
            }
        },
        ("upload", Some(sub_matches)) => {
            let fuzzy_project_path = match sub_matches.value_of("PROJECT") {
                Some(v) => make_path_absolute(Path::new(v)),
                None => std::env::current_dir().unwrap(),
            };

            let security_cookie = sub_matches.value_of("cookie").unwrap();

            let place_id: u64 = {
                let arg = sub_matches.value_of("place_id").unwrap();

                match arg.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        error!("Invalid place ID {}", arg);
                        process::exit(1);
                    },
                }
            };

            let options = commands::UploadOptions {
                fuzzy_project_path,
                security_cookie: security_cookie.to_string(),
                place_id,
            };

            match commands::upload(&options) {
                Ok(_) => {},
                Err(e) => {
                    error!("{}", e);
                    process::exit(1);
                },
            }
        },
        _ => {
            app.print_help().expect("Could not print help text to stdout!");
        },
    }
}