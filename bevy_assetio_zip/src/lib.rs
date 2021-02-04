//! A Bevy [`AssetIo`](bevy::asset::AssetIo) implementation that allows reading from optionally
//! obfuscated zip asset bundles. Using the [`bevy_assetio_zip_bundler`] crate you can also
//! automatically bundle your assets in the desired format in your `build.rs` script.
//!
//! [`bevy_assetio_zip_bundler`]: https://docs.rs/bevy_assetio_zip_bundler
//!
//! # Usage
//!
//! Simply enable the plugin when setting up your Bevy app to enable loading from asset bundle
//! files.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_assetio_zip::{AssetIoZipPlugin, AssetIoZipConfig};
//! App::build()
//!     // Any config must be inserted before adding plugins. This is optional.
//!     .add_resource(AssetIoZipConfig {
//!         // The name of the asset bundle file, excluding the extension, to load
//!         file_name: "assets".into(), // This is the default
//!     })
//!     // Add the default plugins
//!     .add_plugins_with(DefaultPlugins, |group| {
//!         // With our additinoal asset IO plugin
//!         group.add_before::<bevy::asset::AssetPlugin, _>(AssetIoZipPlugin)
//!     })
//!     .run();
//! ```
//!
//! Once enabling the plugin, the game will now search for `assets.zip` and `assets.bin` files
//! adjacent to the executable when attempting to load assets. If an asset is not found in the zip
//! file, it will attempt to load the asset using the default Bevy asset loader for the target
//! platform.
//!
//! # Types of Asset Bundles
//!
//! There are two kinds of asset bundle files supported by this plugin, plain `.zip` files and
//! obfuscated zip files ( which have a `.bin` extension ). Plain `.zip` files are typical zip files
//! that can be created with normal zip software. Obfuscated zip files can be created with
//! [`bevy_assetio_zip_bundler`] and are simply a normal zip file that has had the bytes XOR-ed by
//! `0b01010101`.
//!
//! > **⚠️ WARNING:** Obfuscated zip files provide no real security or protection for your assets.
//! > It is trivial to decript the asset bundle even if it is obfuscated. Obfuscation of the zip is
//! > only a measure to prevent casual users from being able to immediately introspect the data.
//!
//! # Bundling Assets
//!
//! To bundle your bevy assets you can use the [`bevy_assetio_zip_bundler`] crate. The easiest way
//! to use it is to add this to your `build.rs` file:
//!
//! ```ignore
//! fn main() {
//!     bevy_assetio_zip_bundler::bundle_crate_assets();
//! }
//! ```
//!
//! This will automatically zip up your crate's `assets` folder and put it in your `target/` dir
//! when compiling release builds. When distributing your application simply take your asset bundle
//! and place it adjacent to the executable and Bevy will attempt to load assets from the bundle
//! before falling back to the `assets` dir.
//!
//! You can configure the name, obfuscation, and compression of the bundle by creating and
//! `asset_config.toml` file next to your `Cargo.toml` file:
//!
//! ```toml
//! # Bundle assets even for debug builds
//! bundle-for-debug-builds = true # Default: false
//!
//! # Obfuscate assets. This doesn't protect from reverse-engineering, but it makes it a little harder
//! # for the average user to read them.
//! obfuscate = true # Default: false
//!
//! # Compress the asset bundle using Bzip2 compression. Other options are "deflate" and "none".
//! compression = "bzip2" # Default: "bzip2"
//!
//! # The name of the file, not counting the exention, which will be different based on the `obfuscate`
//! # setting. Obfuscated bundles will end in `.bin` and non-obfuscated bundles will end in `.zip`.
//! file-name = "assets" # Default: "assets"
//!
//! # Set the directory that asset bundle should be placed.
//! out-dir = "../target" # Default "./target"
//! ```
//!
//! Alternatively, if you want to create your own tooling or customize the asset bundling process,
//! you can manually bundle the assets using the [`bevy_assetio_zip_bundler::bundle_assets`]
//! function.
//!
//! [`bevy_assetio_zip_bundler::bundle_assets`]:
//! https://docs.rs/bevy_assetio_zip_bundler/latest/bevy_assetio_zip_bundler/fn.bundle_assets.html
//!
//! # Bevy Versions
//! 
//! Supported bevy versions per plugin version:
//!
//! | Bevy Version | Plugin Version                                     |
//! | ------------ | -------------------------------------------------- |
//! | 0.4          | 0.1                                                |
//! | master       | 0.1 with the `bevy-unstable` feature ( see below ) |
//!
//! ## Using Bevy From Master
//!
//! You can use this crate with Bevy master by adding a patch to your `Cargo.toml` and by adding the
//! `bevy-unstable` feature to this crate:
//!
//! ```toml
//! [dependencies]
//! # Bevy version must be set to "0.4" and we will
//! # override it in the patch below.
//! bevy = "0.4"
//! bevy_assetio_zip = { version = "0.1", features = ["bevy-unstable"] }
//!
//! [patch.crates-io]
//! bevy = { git = "https://github.com/bevyengine/bevy.git" }
//! ```
//!
//! Note that as Bevy master may or may not introduce breaking API changes, this crate may or may
//! not compile when using the `bevy-unstable` feature.
//!
//! # License
//!
//! This crate is licensed under the [Katharos License][k_license] which places certain restrictions
//! on what you are allowed to use it for. Please read and understand the terms before using this
//! crate for your project.
//!
//! [k_license]: https://github.com/katharostech/katharos-license

use std::{
    fs::OpenOptions,
    io::{BufReader, Read, Seek},
    path::{Path, PathBuf},
};

use bevy::{
    asset::{AssetIo, AssetIoError},
    prelude::{AppBuilder, AssetServer, Plugin},
    utils::BoxedFuture,
};

use xorio::Xor;
pub use zip::CompressionMethod;
use zip::ZipArchive;

/// Configuration resource fro the [`AssetIoZipPlugin`]
#[derive(Debug, Clone)]
pub struct AssetIoZipConfig {
    /// The name of the assset bundle file to load from, excluding the extension.
    ///
    /// The actual file read will be the filename plus either a `.zip` or a `.bin` extension,
    /// whichever is present. If the `[file_name].zip` file is found it will load the file as a
    /// normal zip, if the `[file_name].bin` file is found, it will attempt to load it as an
    /// obfuscated zip by first XOR-ing the contents of the file by `0b01010101`.
    pub file_name: String,
}

impl Default for AssetIoZipConfig {
    fn default() -> Self {
        Self {
            file_name: "assets".into(),
        }
    }
}

trait FileReader: Read + Seek + Sync + Send {}
impl<T: Read + Seek + Sync + Send> FileReader for T {}

/// A custom [`AssetIo`] implementation that can load assets from an optionally obfuscated zip file
/// and that will fall back to the default asset loader when assets are not found in the zip.
struct AssetIoZip {
    fallback_io: Box<dyn AssetIo>,
    config: AssetIoZipConfig,
}

impl AssetIoZip {
    fn new(fallback_io: Box<dyn AssetIo>, config: AssetIoZipConfig) -> Self {
        // let asset_reader = Self::get_asset_bundle(&config.file_name);
        Self {
            fallback_io,
            config,
            // asset_reader,
        }
    }

    fn bundle(&self) -> Option<ZipArchive<Box<dyn FileReader>>> {
        let exe_dir = std::env::current_exe().expect("Could not obtain current exe path");
        let exe_dir = exe_dir
            .parent()
            .expect("Current exe has no parent dir")
            .to_str()
            .expect("Exe path contains invalid unicode");
        let file_path_bin =
            PathBuf::from(format!("{}/{}.{}", exe_dir, self.config.file_name, "bin"));
        let file_path_zip =
            PathBuf::from(format!("{}/{}.{}", exe_dir, self.config.file_name, "zip"));

        let (path, obfuscate) = if file_path_bin.exists() {
            (file_path_bin, true)
        } else if file_path_zip.exists() {
            (file_path_zip, false)
        } else {
            return None;
        };

        let file = OpenOptions::new().read(true).open(path).ok()?;
        let reader: Box<dyn FileReader> = if obfuscate {
            Box::new(Xor::new(file))
        } else {
            Box::new(file)
        };

        Some(ZipArchive::new(Box::new(BufReader::new(reader)) as Box<dyn FileReader>).ok()?)
    }
}

impl AssetIo for AssetIoZip {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            if let Some(mut asset_bundle) = self.bundle() {
                let has_file = asset_bundle
                    .by_name(path.to_str().expect("non-unicode filename"))
                    .ok()
                    .is_some();
                if has_file {
                    let mut file = asset_bundle
                        .by_name(path.to_str().expect("non-unicode filename"))
                        .unwrap();
                    let mut buf = Vec::with_capacity(file.size() as usize);
                    file.read_to_end(&mut buf)?;

                    Ok(buf)
                } else {
                    self.fallback_io.load_path(path).await
                }
            } else {
                self.fallback_io.load_path(path).await
            }
        })
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        self.fallback_io.read_directory(path)
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.fallback_io.is_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        // Note that we cannot watch for changes inside of the zip file, so we just defer to the
        // default change watcher.
        self.fallback_io.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        // Note that we cannot watch for changes inside of the zip file, so we just defer to the
        // default change watcher.
        self.fallback_io.watch_for_changes()
    }
}

/// An [`AssetIo`] plugin that allows loading Bevy assets from ( optionally ) obfuscated zip files.
pub struct AssetIoZipPlugin;

impl Plugin for AssetIoZipPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // We must get a hold of the task pool in order to create the asset server
        let task_pool = app
            .resources()
            .get::<bevy::tasks::IoTaskPool>()
            .expect("`IoTaskPool` resource not found.")
            .0
            .clone();

        let asset_io = {
            // The platform default asset io requires a reference to the app builder to find its
            // configuration
            let default_assetio = bevy::asset::create_platform_default_asset_io(app);

            let config = app
                .resources()
                .get::<AssetIoZipConfig>()
                .map(|x| (*x).clone())
                .unwrap_or_default();

            // Create the custom asset io instance
            AssetIoZip::new(default_assetio, config)
        };

        // The asset server is constructed and added the resource manager
        #[cfg(feature = "bevy-unstable")]
        app.insert_resource(AssetServer::new(asset_io, task_pool));
        #[cfg(not(feature = "bevy-unstable"))]
        app.add_resource(AssetServer::new(asset_io, task_pool));
    }
}
