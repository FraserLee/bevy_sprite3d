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
    mut commands: Commands,
    //mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let icon = asset_server.load("icon.png");

    // -----------------------------------------------------------------------

    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(0., 0., 5.));

    // ----------------------- Spawn a 3D sprite -----------------------------

    /*
       let custom_material = materials.add(StandardMaterial {
           base_color: Color::srgb(0.5, 0.9, 0.7),
           perceptual_roughness: 0.9,
           reflectance: 0.05,
           ..default()
       });
    */

    commands.spawn((
        Sprite {
            image: icon,
            /*
            color: Color::LinearRgba(LinearRgba {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            }),
            */
            ..default()
        },
        //MeshMaterial3d(custom_material),
        Sprite3d {
            pixels_per_metre: 400.,

            alpha_mode: AlphaMode::Blend,

            unlit: true,

            // pivot: Some(Vec2::new(0.5, 0.5)),
            ..default()
        },
    ));

    // -----------------------------------------------------------------------
}
