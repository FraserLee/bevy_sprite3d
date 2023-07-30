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
    #[asset(texture_atlas(tile_size_x = 24., tile_size_y = 24.))]
    #[asset(texture_atlas(columns = 7, rows = 1))]
    #[asset(path = "gabe-idle-run.png")]
    run: Handle<TextureAtlas>,
}

fn main() {
    App::new()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Ready)
                .with_collection::<ImageAssets>(),
        )
        .add_state(GameState::Loading)
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(Sprite3dPlugin)
        .add_system_set(SystemSet::on_enter(GameState::Ready).with_system(setup))
        .add_system_set(SystemSet::on_update(GameState::Ready).with_system(animate_sprite))
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn setup(mut commands: Commands, images: Res<ImageAssets>, mut sprite_params: Sprite3dParams) {
    commands
        .spawn(Camera3dBundle::default())
        .insert(Transform::from_xyz(0., 0., 5.));

    // -------------------- Spawn a 3D atlas sprite --------------------------

    commands
        .spawn(
            AtlasSprite3d {
                atlas: images.run.clone(),

                pixels_per_meter: 32.,
                partial_alpha: true,
                unlit: true,

                index: 3,

                // transform: Transform::from_xyz(0., 0., 0.),
                // pivot: Some(Vec2::new(0.5, 0.5)),
                ..default()
            }
            .bundle(&mut sprite_params),
        )
        .insert(AnimationTimer(Timer::from_seconds(
            0.1,
            TimerMode::Repeating,
        )));

    // -----------------------------------------------------------------------
}

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut AtlasSprite3dComponent)>,
) {
    for (mut timer, mut sprite) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            sprite.index = (sprite.index + 1) % sprite.atlas.len();
        }
    }
}
