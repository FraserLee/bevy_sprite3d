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
struct Assets(Handle<Image>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Sprite3dPlugin)
        .add_state::<GameState>()
        // initially load assets
        .add_systems(
            Startup,
            |asset_server: Res<AssetServer>, mut assets: ResMut<Assets>| {
                assets.0 = asset_server.load("icon.png");
            },
        )
        // run `setup` every frame while loading. Once it detects the right
        // conditions it'll switch to the next state.
        .add_systems(Update, setup.run_if(in_state(GameState::Loading)))
        .insert_resource(Assets::default())
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    assets: Res<Assets>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut sprite_params: Sprite3dParams,
) {
    // poll every frame to check if assets are loaded. Once they are, we can proceed with setup.
    if asset_server.get_load_state(assets.0.clone()) != LoadState::Loaded {
        return;
    }

    next_state.set(GameState::Ready);

    // -----------------------------------------------------------------------

    commands
        .spawn(Camera3dBundle::default())
        .insert(Transform::from_xyz(0., 0., 5.));

    // ----------------------- Spawn a 3D sprite -----------------------------

    commands.spawn(
        Sprite3d {
            image: assets.0.clone(),

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
