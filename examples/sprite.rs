use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Sprite3dPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut billboards: ResMut<Assets<Billboard>>,
    mut commands: Commands,
) {
    let image: Handle<Image> = asset_server.load("icon.png");
    let billboard = Billboard::new(image.clone(), 400., None, false);
    let material = StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    };

    // Spawn the camera
    commands.spawn(Camera3d::default()).insert(Transform::from_xyz(0., 0., 5.));

    // Spawn a 3D sprite
    commands.spawn((
        Sprite3d::default(),
        Sprite3dBillboard::from(billboards.add(billboard)),
        MeshMaterial3d(materials.add(material)),
    ));
}
