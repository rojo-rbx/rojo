use std::{
    env, panic,
    path::{Path, PathBuf},
    process,
};

use clap::{clap_app, ArgMatches};
use log::error;

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
    let app = clap_app!(Rojo =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: env!("CARGO_PKG_DESCRIPTION"))

        (@arg verbose: --verbose -v +multiple +global "Sets verbosity level. Can be specified multiple times.")

        (@subcommand init =>
            (about: "Creates a new Rojo project.")
            (@arg PATH: "Path to the place to create the project. Defaults to the current directory.")
            (@arg kind: --kind +takes_value "The kind of project to create, 'place' or 'model'. Defaults to place.")
        )

        (@subcommand serve =>
            (about: "Serves the project's files for use with the Rojo Studio plugin.")
            (@arg PROJECT: "Path to the project to serve. Defaults to the current directory.")
            (@arg port: --port +takes_value "The port to listen on. Defaults to 34872.")
        )

        (@subcommand build =>
            (about: "Generates a model or place file from the project.")
            (@arg PROJECT: "Path to the project to serve. Defaults to the current directory.")
            (@arg output: --output -o +takes_value +required "Where to output the result.")
        )

        (@subcommand upload =>
            (about: "Generates a place or model file out of the project and uploads it to Roblox.")
            (@arg PROJECT: "Path to the project to upload. Defaults to the current directory.")
            (@arg kind: --kind +takes_value "The kind of asset to generate, 'place', or 'model'. Defaults to place.")
            (@arg cookie: --cookie +takes_value "Authenication cookie to use. If not specified, Rojo will attempt to find one from the system automatically.")
            (@arg asset_id: --asset_id +takes_value +required "Asset ID to upload to.")
        )
    );

    let matches = app.get_matches();

    {
        let verbosity = matches.occurrences_of("verbose");
        let log_filter = match verbosity {
            0 => "warn",
            1 => "warn,librojo=info",
            2 => "warn,librojo=trace",
            _ => "trace",
        };

        let log_env = env_logger::Env::default().default_filter_or(log_filter);

        env_logger::Builder::from_env(log_env)
            .default_format_timestamp(false)
            .init();
    }

    let result = panic::catch_unwind(|| match matches.subcommand() {
        ("init", Some(sub_matches)) => start_init(sub_matches),
        ("serve", Some(sub_matches)) => start_serve(sub_matches),
        ("build", Some(sub_matches)) => start_build(sub_matches),
        ("upload", Some(sub_matches)) => start_upload(sub_matches),
        _ => eprintln!("Usage: rojo <SUBCOMMAND>\nUse 'rojo help' for more help."),
    });

    if let Err(error) = result {
        let message = match error.downcast_ref::<&str>() {
            Some(message) => message.to_string(),
            None => match error.downcast_ref::<String>() {
                Some(message) => message.clone(),
                None => "<no message>".to_string(),
            },
        };

        show_crash_message(&message);
        process::exit(1);
    }
}

fn show_crash_message(message: &str) {
    error!("Rojo crashed!");
    error!("This is a bug in Rojo.");
    error!("");
    error!("Please consider filing a bug: https://github.com/rojo-rbx/rojo/issues");
    error!("");
    error!("Details: {}", message);
}

fn start_init(sub_matches: &ArgMatches) {
    let fuzzy_project_path =
        make_path_absolute(Path::new(sub_matches.value_of("PATH").unwrap_or("")));
    let kind = sub_matches.value_of("kind");

    let options = commands::InitOptions {
        fuzzy_project_path,
        kind,
    };

    match commands::init(&options) {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }
}

fn start_serve(sub_matches: &ArgMatches) {
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
            }
        },
        None => None,
    };

    let options = commands::ServeOptions {
        fuzzy_project_path,
        port,
    };

    match commands::serve(&options) {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }
}

fn start_build(sub_matches: &ArgMatches) {
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
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }
}

fn start_upload(sub_matches: &ArgMatches) {
    let fuzzy_project_path = match sub_matches.value_of("PROJECT") {
        Some(v) => make_path_absolute(Path::new(v)),
        None => std::env::current_dir().unwrap(),
    };

    let kind = sub_matches.value_of("kind");
    let auth_cookie = sub_matches.value_of("cookie").map(Into::into);

    let asset_id: u64 = {
        let arg = sub_matches.value_of("asset_id").unwrap();

        match arg.parse() {
            Ok(v) => v,
            Err(_) => {
                error!("Invalid place ID {}", arg);
                process::exit(1);
            }
        }
    };

    let options = commands::UploadOptions {
        fuzzy_project_path,
        auth_cookie,
        asset_id,
        kind,
    };

    match commands::upload(options) {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    }
}
