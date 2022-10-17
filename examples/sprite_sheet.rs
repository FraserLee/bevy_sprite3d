use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use bevy_sprite3d::*;
use bevy_asset_loader::prelude::*;


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum GameState { Loading, Ready }

#[derive(AssetCollection)]
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
                .with_collection::<ImageAssets>()
        )
        .insert_resource(ImageSettings::default_nearest())
        .add_state(GameState::Loading)
        .add_plugins(DefaultPlugins)
        .add_plugin(Sprite3dPlugin)
        .add_system_set( SystemSet::on_enter(GameState::Ready).with_system(setup) )
        .run();

}


fn setup(
    mut commands: Commands, 
    images: Res<ImageAssets>,
    mut sprite_params: Sprite3dParams
) {

    commands.spawn_bundle(Camera3dBundle::default())
            .insert(Transform::from_xyz(0., 0., 5.));

    // -------------------- Spawn a 3D atlas sprite --------------------------

    commands.spawn_bundle(AtlasSprite3d {
            atlas: images.run.clone(),

            pixels_per_metre: 32.,
            partial_alpha: true,
            unlit: true,

            index: 3,

            // transform: Transform::from_xyz(0., 0., 0.),
            // pivot: Some(Vec2::new(0.5, 0.5)),

            ..default()
    }.bundle(&mut sprite_params));

    // -----------------------------------------------------------------------
}

