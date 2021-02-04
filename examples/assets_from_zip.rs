use bevy::prelude::*;
use bevy_advanced_assets::{AdvancedAssetIoConfig, AdvancedAssetIoPlugin};

fn main() {
    App::build()
        // Config must be inserted before adding plugins
        .insert_resource(AdvancedAssetIoConfig::default())
        // Add the default plugins
        .add_plugins_with(DefaultPlugins, |group| {
            // With our additinoal asset IO plugin
            group.add_before::<bevy::asset::AssetPlugin, _>(AdvancedAssetIoPlugin)
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
    commands
        .spawn(OrthographicCameraBundle::new_2d())
        .spawn(SpriteBundle {
            material: materials.add(texture_handle.into()),
            transform: Transform::from_scale(Vec3::splat(10.0)),
            ..Default::default()
        });
}
