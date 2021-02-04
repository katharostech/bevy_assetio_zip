use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Seek},
    path::{Path, PathBuf},
};

use bevy::{
    asset::{AssetIo, AssetIoError},
    prelude::{AppBuilder, AssetServer, Plugin},
    utils::BoxedFuture,
};

mod xor;

use xor::Xor;
pub use zip::CompressionMethod;
use zip::ZipArchive;

/// Configuration resource fro the [`AdvancedAssetIoPlugin`]
#[derive(Debug, Clone)]
pub struct AdvancedAssetIoConfig {
    /// The name of the assset bundle file to load from, excluding the extension.
    ///
    /// The actual file read will be the filename plus either a `.zip` or a `.bin` extension,
    /// whichever is present. If the `[file_name].zip` file is found it will load the file as a
    /// normal zip, if the `[file_name].bin` file is found, it will attempt to load it as an
    /// obfuscated zip by first XOR-ing the contents of the file by `0b01010101`.
    file_name: String,
}

impl Default for AdvancedAssetIoConfig {
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
struct AdvancedAssetIo {
    fallback_io: Box<dyn AssetIo>,
    config: AdvancedAssetIoConfig,
}

impl AdvancedAssetIo {
    fn new(fallback_io: Box<dyn AssetIo>, config: AdvancedAssetIoConfig) -> Self {
        // let asset_reader = Self::get_asset_bundle(&config.file_name);
        Self {
            fallback_io,
            config,
            // asset_reader,
        }
    }

    fn bundle(&self) -> Option<ZipArchive<Box<dyn FileReader>>> {
        let file_path_bin = PathBuf::from(format!("{}.{}", self.config.file_name, "bin"));
        let file_path_zip = PathBuf::from(format!("{}.{}", self.config.file_name, "zip"));

        let (path, obfuscate) = if file_path_bin.exists() {
            (file_path_bin, true)
        } else if file_path_zip.exists() {
            (file_path_zip, false)
        } else {
            return None;
        };

        let file = OpenOptions::new().read(true).open(path).ok()?;
        let reader: Box<dyn FileReader> = if obfuscate {
            Box::new(Xor(file))
        } else {
            Box::new(file)
        };

        Some(ZipArchive::new(Box::new(BufReader::new(reader)) as Box<dyn FileReader>).ok()?)
    }
}

impl AssetIo for AdvancedAssetIo {
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
pub struct AdvancedAssetIoPlugin;

impl Plugin for AdvancedAssetIoPlugin {
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
                .get::<AdvancedAssetIoConfig>()
                .map(|x| (*x).clone())
                .unwrap_or_default();

            // Create the custom asset io instance
            AdvancedAssetIo::new(default_assetio, config)
        };

        // The asset server is constructed and added the resource manager
        app.insert_resource(AssetServer::new(asset_io, task_pool));
    }
}
