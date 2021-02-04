use std::{fs::File, io::BufWriter, path::Path};

use serde::Deserialize;
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

include!("src/xor.rs");

#[derive(Debug, Deserialize)]
enum Compression {
    None,
    Bzip2,
    Deflate,
}

impl Into<CompressionMethod> for Compression {
    fn into(self) -> CompressionMethod {
        match self {
            Compression::None => CompressionMethod::Stored,
            Compression::Bzip2 => CompressionMethod::Bzip2,
            Compression::Deflate => CompressionMethod::Deflated,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
struct AssetBundlerConfig {
    file_name: String,
    compression: Compression,
    obfuscate: bool,
    bundle_for_debug_builds: bool,
}

impl Default for AssetBundlerConfig {
    fn default() -> Self {
        Self {
            file_name: "assets".into(),
            compression: Compression::Bzip2,
            obfuscate: false,
            bundle_for_debug_builds: false,
        }
    }
}

const CONFIG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/asset_config.toml");

fn main() {
    // Load bundler config file
    let config: AssetBundlerConfig = std::fs::read(CONFIG_PATH)
        .and_then(
            |x| Ok(toml::from_slice(x.as_slice()).expect("Could not parse asset_config.toml")),
        )
        .unwrap_or_default();

    let file_extension = if config.obfuscate { "bin" } else { "zip" };
    let profile = std::env::var("PROFILE").unwrap();

    if profile == "release" || config.bundle_for_debug_builds == true {
        // Bundle assets
        zip_dir(
            &concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(),
            &format!("target/{}/{}.{}", profile, config.file_name, file_extension),
            config.compression.into(),
            config.obfuscate,
        );
    }
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
        Box::new(Xor(archive_file))
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
