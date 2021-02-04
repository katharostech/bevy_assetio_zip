# bevy_assetio_zip

[![Crates.io](https://img.shields.io/crates/v/bevy_assetio_zip)](https://crates.io/crates/bevy_assetio_zip)
[![Docs.rs](https://docs.rs/bevy_assetio_zip/badge.svg)](https://docs.rs/bevy_assetio_zip)
[![Katharos License](https://img.shields.io/badge/License-Katharos-blue)](https://github.com/katharostech/katharos-license)

A Bevy [`AssetIo`](bevy::asset::AssetIo) implementation that allows reading from optionally
obfuscated zip asset bundles. Using the [`bevy_assetio_zip_bundler`] crate you can also
automatically bundle your assets in the desired format in your `build.rs` script.

[`bevy_assetio_zip_bundler`]: https://docs.rs/bevy_assetio_zip_bundler

## Usage

Simply enable the plugin when setting up your Bevy app to enable loading from asset bundle
files.

```rust
App::build()
    // Any config must be inserted before adding plugins. This is optional.
    .insert_resource(AssetIoZipConfig {
        // The name of the asset bundle file, excluding the extension, to load
        file_name: "assets".into() # This is the default
    })
    // Add the default plugins
    .add_plugins_with(DefaultPlugins, |group| {
        // With our additinoal asset IO plugin
        group.add_before::<bevy::asset::AssetPlugin, _>(AssetIoZipPlugin)
    })
    .run();
```

Once enabling the plugin, the game will now search for `assets.zip` and `assets.bin` files
adjacent to the executable when attempting to load assets. If an asset is not found in the zip
file, it will attempt to load the asset using the default Bevy asset loader for the target
platform.

## Types of Asset Bundles

There are two kinds of asset bundle files supported by this plugin, plain `.zip` files and
obfuscated zip files ( which have a `.bin` extension ). Plain `.zip` files are typical zip files
that can be created with normal zip software. Obfuscated zip files can be created with
[`bevy_assetio_zip_bundler`] and are simply a normal zip file that has had the bytes XOR-ed by
`0b01010101`.

> **⚠️ WARNING:** Obfuscated zip files provide no real security or protection for your assets.
> It is trivial to decript the asset bundle even if it is obfuscated. Obfuscation of the zip is
> only a measure to prevent casual users from being able to immediately introspect the data.

## Bundling Assets

To bundle your bevy assets you can use the [`bevy_assetio_zip_bundler`] crate. The easiest way
to use it is to add this to your `build.rs` file:

```rust
fn main() {
    bevy_assetio_zip_bundler::bundle_crate_assets();
}
```

This will automatically zip up your crate's `assets` folder and put it in your `target/` dir
when compiling release builds. You can configure the name, obfuscation, and compression of the
bundle by creating and `asset_config.toml` file next to your `Cargo.toml` file:

```toml
# Bundle assets even for debug builds
bundle-for-debug-builds = true # Default: false

# Obfuscate assets. This doesn't protect from reverse-engineering, but it makes it a little harder
# for the average user to read them.
obfuscate = true # Default: false

# Compress the asset bundle using Bzip2 compression. Other options are "deflate" and "none".
compression = "bzip2" # Default: "bzip2"

# The name of the file, not counting the exention, which will be different based on the `obfuscate`
# setting. Obfuscated bundles will end in `.bin` and non-obfuscated bundles will end in `.zip`.
file-name = "assets" # Default: "assets"

# Set the directory that asset bundle should be placed.
out-dir = "../target" # Default "./target"
```

Alternatively, if you want to create your own tooling or customize the asset bundling process,
you can manually bundle the assets using the [`bevy_assetio_zip_bundler::bundle_assets`]
function.

[`bevy_assetio_zip_bundler::bundle_assets`]:
https://docs.rs/bevy_assetio_zip_bundler/latest/bevy_assetio_zip_bundler/fn.bundle_assets.html

## License

This crate is licensed under the [Katharos License][k_license] which places certain restrictions
on what you are allowed to use it for. Please read and understand the terms before using this
crate for your project.

[k_license]: https://github.com/katharostech/katharos-license

[`Read`]: https://doc.rust-lang.org/stable/std/io/trait.Read.html
[`Write`]: https://doc.rust-lang.org/stable/std/io/trait.Write.html
