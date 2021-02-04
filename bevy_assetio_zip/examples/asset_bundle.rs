use bevy::prelude::*;
use bevy_assetio_zip::{AssetIoZipConfig, AssetIoZipPlugin};

fn main() {
    let mut builder = App::build();

    // Config must be inserted before adding plugins
    #[cfg(feature = "bevy-unstable")]
    builder.insert_resource(AssetIoZipConfig::default());
    #[cfg(not(feature = "bevy-unstable"))]
    builder.add_resource(AssetIoZipConfig::default());

    // Add the default plugins
    builder
        .add_plugins_with(DefaultPlugins, |group| {
            // With our additinoal asset IO plugin
            group.add_before::<bevy::asset::AssetPlugin, _>(AssetIoZipPlugin)
        })
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("sensei.png");
    #[cfg(feature = "bevy-unstable")]
    commands.spawn(OrthographicCameraBundle::new_2d());

    #[cfg(not(feature = "bevy-unstable"))]
    commands.spawn(Camera2dBundle::default());

    commands.spawn(SpriteBundle {
        material: materials.add(texture_handle.into()),
        transform: Transform::from_scale(Vec3::splat(10.0)),
        ..Default::default()
    });
}
