use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy_sprite3d::*;

#[derive(States, Hash, Clone, PartialEq, Eq, Debug, Default)]
enum GameState {
    #[default]
    Loading,
    Ready,
}

#[derive(Resource, Default)]
struct ImageAssets {
    image: Handle<Image>, // the `image` field here is only used to query the load state, lots of the
    atlas: Handle<TextureAtlas>, // code in this file disappears if something like bevy_asset_loader is used.
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(Sprite3dPlugin)
        .add_state::<GameState>()
        // initially load assets
        .add_systems(
            Startup,
            |asset_server: Res<AssetServer>,
             mut assets: ResMut<ImageAssets>,
             mut texture_atlases: ResMut<Assets<TextureAtlas>>| {
                assets.image = asset_server.load("gabe-idle-run.png");

                assets.atlas = texture_atlases.add(TextureAtlas::from_grid(
                    assets.image.clone(),
                    Vec2::new(24.0, 24.0),
                    7,
                    1,
                    None,
                    None,
                ));
            },
        )
        // run `setup` every frame while loading. Once it detects the right
        // conditions it'll switch to the next state.
        .add_systems(Update, setup.run_if(in_state(GameState::Loading)))
        // every frame, animate the sprite
        .add_systems(Update, animate_sprite.run_if(in_state(GameState::Ready)))
        .insert_resource(ImageAssets::default())
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    assets: Res<ImageAssets>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut sprite_params: Sprite3dParams,
) {
    // poll every frame to check if assets are loaded. Once they are, we can proceed with setup.
    if asset_server.get_load_state(assets.image.clone()) != LoadState::Loaded {
        return;
    }

    next_state.set(GameState::Ready);

    // -----------------------------------------------------------------------

    commands
        .spawn(Camera3dBundle::default())
        .insert(Transform::from_xyz(0., 0., 5.));

    // -------------------- Spawn a 3D atlas sprite --------------------------

    commands
        .spawn(
            AtlasSprite3d {
                atlas: assets.atlas.clone(),

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
