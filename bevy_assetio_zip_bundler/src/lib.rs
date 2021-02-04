//! Asset bundler meant for use by the [`bevy_assetio_zip`] crate. See [`bevy_assetio_zip`] for usage.
//!
//! [`bevy_assetio_zip`]: https://docs.rs/bevy_assetio_zip
//!
//! # License
//!
//! This crate is licensed under the [Katharos License][k_license] which places certain
//! restrictions on what you are allowed to use it for. Please read and understand the terms before
//! using this crate for your project.
//!
//! [k_license]: https://github.com/katharostech/katharos-license

#[cfg(feature = "bundle-crate-assets")]
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufWriter, Read, Seek, Write},
    path::Path,
};

#[cfg(feature = "bundle-crate-assets")]
use serde::Deserialize;
use walkdir::WalkDir;
use xorio::Xor;
pub use zip::CompressionMethod;
use zip::{write::FileOptions, ZipWriter};

/// Compression mode to use for asset bundle
#[cfg(feature = "bundle-crate-assets")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Compression {
    None,
    Bzip2,
    Deflate,
}

#[cfg(feature = "bundle-crate-assets")]
impl Into<CompressionMethod> for Compression {
    fn into(self) -> CompressionMethod {
        match self {
            Compression::None => CompressionMethod::Stored,
            Compression::Bzip2 => CompressionMethod::Bzip2,
            Compression::Deflate => CompressionMethod::Deflated,
        }
    }
}

/// Configuration options for the `asset_config.toml` file
#[cfg(feature = "bundle-crate-assets")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct AssetBundlerConfig {
    file_name: String,
    compression: Compression,
    obfuscate: bool,
    bundle_for_debug_builds: bool,
    out_dir: String,
}

#[cfg(feature = "bundle-crate-assets")]
impl Default for AssetBundlerConfig {
    fn default() -> Self {
        Self {
            file_name: "assets".into(),
            compression: Compression::Bzip2,
            obfuscate: false,
            bundle_for_debug_builds: false,
            out_dir: "./target".into(),
        }
    }
}

/// Automatically bundle the assets from this crate's `assets` dir and parse the bundler config from
/// the optional `asset_config.toml` file.
///
/// This function is meant to be used in your crates `build.rs` file.
#[cfg(feature = "bundle-crate-assets")]
pub fn bundle_crate_assets() {
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let config_path = PathBuf::from(cargo_dir.clone()).join("asset_config.toml");

    // Load bundler config file
    let config: AssetBundlerConfig = std::fs::read(config_path)
        .and_then(
            |x| Ok(toml::from_slice(x.as_slice()).expect("Could not parse asset_config.toml")),
        )
        .unwrap_or_default();

    let profile = std::env::var("PROFILE").unwrap();
    let file_extension = if config.obfuscate { "bin" } else { "zip" };
    let asset_dir = PathBuf::from(cargo_dir).join("assets");
    let bundle_file = format!("{}/{}.{}", config.out_dir, config.file_name, file_extension).into();
    std::fs::create_dir_all(config.out_dir).unwrap();

    if profile == "release" || config.bundle_for_debug_builds == true {
        bundle_assets(
            asset_dir,
            bundle_file,
            config.obfuscate,
            config.compression.into(),
        );
    }
}

/// Bundle the assets in the given `asset_dir` and write the result to `bundle_file`.
pub fn bundle_assets<P: AsRef<Path>>(
    asset_dir: P,
    bundle_file: P,
    obfuscate: bool,
    compression: CompressionMethod,
) {
    // Bundle assets
    zip_dir(
        asset_dir.as_ref(),
        bundle_file.as_ref(),
        compression.into(),
        obfuscate,
    );
}

trait WriteSeek: Seek + Write {}
impl<T: Seek + Write> WriteSeek for T {}

fn zip_dir<P: AsRef<Path>>(
    source_dir: P,
    target_file: P,
    compression: CompressionMethod,
    obfuscate: bool,
) {
    let source_dir = source_dir.as_ref();
    let walkdir = WalkDir::new(source_dir);
    let archive_file = File::create(target_file.as_ref()).expect("Could not create archive file");
    let writer: Box<dyn WriteSeek> = if obfuscate {
        Box::new(Xor::new(archive_file))
    } else {
        Box::new(archive_file)
    };
    let buf_writer = BufWriter::new(writer);

    let mut zip = ZipWriter::new(buf_writer);
    let options = FileOptions::default().compression_method(compression);

    let mut buffer = Vec::new();
    for entry in walkdir {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.strip_prefix(source_dir).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            #[allow(deprecated)]
            zip.start_file_from_path(name, options).unwrap();
            let mut f = File::open(path).unwrap();

            f.read_to_end(&mut buffer).unwrap();
            zip.write_all(&*buffer).unwrap();
            buffer.clear();
        } else if name.as_os_str().len() != 0 {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options).unwrap();
        }
    }

    zip.finish().unwrap();
}
