use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

#[derive(States, Hash, Clone, PartialEq, Eq, Debug, Default)]
enum GameState { #[default] Loading, Ready }

#[derive(Resource, Default)]
struct Assets(Handle<Image>);

fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Sprite3dPlugin)
        .init_state::<GameState>()

        // initially load assets
        .add_systems( Startup, |asset_server: Res<AssetServer>, mut assets: ResMut<Assets>| {
            assets.0 = asset_server.load("icon.png");
        } )

        // run `setup` every frame while loading. Once it detects the right
        // conditions it'll switch to the next state.
        .add_systems( Update, setup.run_if(in_state(GameState::Loading)) )

        .insert_resource(Assets::default())
        .run();

}

fn setup(
    asset_server      : Res<AssetServer>,
    assets            : Res<Assets>,
    mut commands      : Commands,
    mut next_state    : ResMut<NextState<GameState>>,
) {

    // poll every frame to check if assets are loaded. Once they are, we can proceed with setup.
    if !asset_server.get_load_state(assets.0.id()).is_some_and(|s| s.is_loaded()) { return; }

    next_state.set(GameState::Ready);

    // -----------------------------------------------------------------------

    commands.spawn(Camera3d::default()).insert(Transform::from_xyz(0., 0., 5.));

    // ----------------------- Spawn a 3D sprite -----------------------------

    commands.spawn((
        Sprite3d {
            pixels_per_metre: 400.,

            alpha_mode: AlphaMode::Blend,

            unlit: true,

            // pivot: Some(Vec2::new(0.5, 0.5)),

            ..default()
        },
        Sprite {
            image: assets.0.clone(),
            ..default()
        }
    ));

    // -----------------------------------------------------------------------
}
