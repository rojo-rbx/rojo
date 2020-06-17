use std::{
    fs::File,
    io::{BufWriter, Write},
};

use memofs::Vfs;
use thiserror::Error;
use tokio::runtime::Runtime;

use crate::{cli::BuildCommand, serve_session::ServeSession, snapshot::RojoTree};

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

#[derive(Debug, Error)]
enum Error {
    #[error("Could not detect what kind of file to build. Expected output file to end in .rbxl, .rbxlx, .rbxm, or .rbxmx.")]
    UnknownOutputKind,
}

fn xml_encode_config() -> rbx_xml::EncodeOptions {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

pub fn build(options: BuildCommand) -> Result<(), anyhow::Error> {
    log::trace!("Constructing in-memory filesystem");

    let vfs = Vfs::new_default();
    vfs.set_watch_enabled(options.watch);

    let session = ServeSession::new(vfs, &options.absolute_project())?;
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

fn write_model(tree: &RojoTree, options: &BuildCommand) -> Result<(), anyhow::Error> {
    let output_kind = detect_output_kind(&options).ok_or(Error::UnknownOutputKind)?;
    log::debug!("Hoping to generate file of type {:?}", output_kind);

    let root_id = tree.get_root_id();

    log::trace!("Opening output file for write");
    let file = File::create(&options.output)?;
    let mut file = BufWriter::new(file);

    match output_kind {
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            rbx_xml::to_writer(&mut file, tree.inner(), &[root_id], xml_encode_config())?;
        }
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // RbxTree representation does.

            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_xml::to_writer(&mut file, tree.inner(), top_level_ids, xml_encode_config())?;
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

    file.flush()?;

    let filename = options
        .output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid utf-8>");
    log::info!("Built project to {}", filename);

    Ok(())
}
