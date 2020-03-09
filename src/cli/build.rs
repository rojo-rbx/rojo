use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

use snafu::{ResultExt, Snafu};
use tokio::runtime::Runtime;

use crate::{
    cli::BuildCommand,
    project::ProjectError,
    serve_session::ServeSession,
    snapshot::RojoTree,
    vfs::{RealFetcher, Vfs, WatchMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputKind {
    Rbxmx,
    Rbxlx,
    Rbxm,
    Rbxl,
}

fn detect_output_kind(options: &BuildCommand) -> Option<OutputKind> {
    let extension = options.output.extension()?.to_str()?;

    match extension {
        "rbxlx" => Some(OutputKind::Rbxlx),
        "rbxmx" => Some(OutputKind::Rbxmx),
        "rbxl" => Some(OutputKind::Rbxl),
        "rbxm" => Some(OutputKind::Rbxm),
        _ => None,
    }
}

#[derive(Debug, Snafu)]
pub struct BuildError(Error);

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Could not detect what kind of file to create"))]
    UnknownOutputKind,

    #[snafu(display("{}", source))]
    Io { source: io::Error },

    #[snafu(display("{}", source))]
    XmlModelEncode { source: rbx_xml::EncodeError },

    #[snafu(display("Binary model error: {:?}", source))]
    BinaryModelEncode {
        #[snafu(source(false))]
        source: rbx_binary::EncodeError,
    },

    #[snafu(display("{}", source))]
    Project { source: ProjectError },
}

impl From<rbx_binary::EncodeError> for Error {
    fn from(source: rbx_binary::EncodeError) -> Self {
        Error::BinaryModelEncode { source }
    }
}

fn xml_encode_config() -> rbx_xml::EncodeOptions {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

pub fn build(options: BuildCommand) -> Result<(), BuildError> {
    Ok(build_inner(options)?)
}

fn build_inner(options: BuildCommand) -> Result<(), Error> {
    log::trace!("Constructing in-memory filesystem");

    let watch_mode = if options.watch {
        WatchMode::Enabled
    } else {
        WatchMode::Disabled
    };

    let vfs = Vfs::new(RealFetcher::new(watch_mode));

    let session = ServeSession::new(vfs, &options.absolute_project());
    let mut cursor = session.message_queue().cursor();

    {
        let tree = session.tree();
        write_model(&tree, &options)?;
    }

    if options.watch {
        let mut rt = Runtime::new().unwrap();

        loop {
            let receiver = session.message_queue().subscribe(cursor);
            let (new_cursor, _patch_set) = rt.block_on(receiver).unwrap();
            cursor = new_cursor;

            let tree = session.tree();
            write_model(&tree, &options)?;
        }
    }

    Ok(())
}

fn write_model(tree: &RojoTree, options: &BuildCommand) -> Result<(), Error> {
    let output_kind = detect_output_kind(&options).ok_or(Error::UnknownOutputKind)?;
    log::debug!("Hoping to generate file of type {:?}", output_kind);

    let root_id = tree.get_root_id();

    log::trace!("Opening output file for write");
    let file = File::create(&options.output).context(Io)?;
    let mut file = BufWriter::new(file);

    match output_kind {
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            rbx_xml::to_writer(&mut file, tree.inner(), &[root_id], xml_encode_config())
                .context(XmlModelEncode)?;
        }
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // RbxTree representation does.

            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_xml::to_writer(&mut file, tree.inner(), top_level_ids, xml_encode_config())
                .context(XmlModelEncode)?;
        }
        OutputKind::Rbxm => {
            rbx_binary::encode(tree.inner(), &[root_id], &mut file)?;
        }
        OutputKind::Rbxl => {
            log::warn!("Support for building binary places (rbxl) is still experimental.");
            log::warn!("Using the XML place format (rbxlx) is recommended instead.");
            log::warn!("For more info, see https://github.com/LPGhatguy/rojo/issues/180");

            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_binary::encode(tree.inner(), top_level_ids, &mut file)?;
        }
    }

    file.flush().context(Io)?;

    Ok(())
}
