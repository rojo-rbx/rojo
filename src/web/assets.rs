//! A simple asset reloading system intended for iterating on CSS. When in the
//! `dev_live_assets` feature is enabled, files are read from disk and watched
//! for changes. Otherwise, they're baked into the executable.

macro_rules! declare_asset {
    ($name: ident, $path: expr) => {
        pub fn $name() -> &'static str {
            if cfg!(feature = "dev_live_assets") {
                use std::{fs::read_to_string, path::Path};

                let file_path = Path::new(file!());
                let asset_path = file_path.parent().unwrap().join($path);

                println!("Reloading {}", asset_path.display());

                let content = read_to_string(asset_path)
                    .expect("Couldn't read dev live asset")
                    .into_boxed_str();

                Box::leak(content)
            } else {
                static CONTENT: &str = include_str!($path);
                CONTENT
            }
        }
    };
}

declare_asset!(css, "../../assets/index.css");

pub fn logo() -> &'static [u8] {
    static LOGO: &[u8] = include_bytes!("../../assets/brand_images/logo-512.png");

    LOGO
}

pub fn icon() -> &'static [u8] {
    static ICON: &[u8] = include_bytes!("../../assets/brand_images/icon-32.png");

    ICON
}
