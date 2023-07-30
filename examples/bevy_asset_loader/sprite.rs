// the following example is written for bevy 0.9
// todo: upgrade once bevy_asset_loader is updated

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_sprite3d::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum GameState {
    Loading,
    Ready,
}

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "icon.png")]
    icon: Handle<Image>,
}

fn main() {
    App::new()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Ready)
                .with_collection::<ImageAssets>(),
        )
        .add_state(GameState::Loading)
        .add_plugins(DefaultPlugins)
        .add_plugin(Sprite3dPlugin)
        .add_system_set(SystemSet::on_enter(GameState::Ready).with_system(setup))
        .run();
}

fn setup(mut commands: Commands, images: Res<ImageAssets>, mut sprite_params: Sprite3dParams) {
    commands
        .spawn(Camera3dBundle::default())
        .insert(Transform::from_xyz(0., 0., 5.));

    // ----------------------- Spawn a 3D sprite -----------------------------

    commands.spawn(
        Sprite3d {
            image: images.icon.clone(),

            pixels_per_meter: 400.,

            partial_alpha: true,

            unlit: true,

            // transform: Transform::from_xyz(0., 0., 0.),
            // pivot: Some(Vec2::new(0.5, 0.5)),
            ..default()
        }
        .bundle(&mut sprite_params),
    );

    // -----------------------------------------------------------------------
}
