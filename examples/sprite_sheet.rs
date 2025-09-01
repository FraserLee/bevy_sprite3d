use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

#[derive(Resource, Default)]
struct ImageAssets {
    image: Handle<Image>, // the `image` field here is only used to query the load state, lots of the
    layout: Handle<TextureAtlasLayout>, // code in this file disappears if something like bevy_asset_loader is used.
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(Sprite3dPlugin)
        // initially load assets
        .add_systems(
            Startup,
            |asset_server: Res<AssetServer>,
             mut assets: ResMut<ImageAssets>,
             mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>| {
                assets.image = asset_server.load("gabe-idle-run.png");
                assets.layout = texture_atlases.add(TextureAtlasLayout::from_grid(
                    UVec2::new(24, 24),
                    7,
                    1,
                    None,
                    None,
                ));
            },
        )
        .add_systems(Startup, setup)
        // every frame, animate the sprite
        .add_systems(Update, animate_sprite)
        .insert_resource(ImageAssets::default())
        .run();
}

fn setup(assets: Res<ImageAssets>, mut commands: Commands) {
    // -----------------------------------------------------------------------

    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(0., 0., 5.));

    // -------------------- Spawn a 3D atlas sprite --------------------------

    let texture_atlas = TextureAtlas {
        layout: assets.layout.clone(),
        index: 3,
    };

    commands.spawn((
        Sprite {
            image: assets.image.clone(),
            texture_atlas: Some(texture_atlas),
            ..default()
        },
        Sprite3d {
            pixels_per_metre: 32.,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            // pivot: Some(Vec2::new(0.5, 0.5)),
            ..default()
        },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    // -----------------------------------------------------------------------
}

fn animate_sprite(
    time: Res<Time>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    mut query: Query<(&mut AnimationTimer, &mut Sprite)>,
) {
    for (mut timer, mut sprite) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            let atlas = sprite.texture_atlas.as_mut().unwrap();
            if let Some(layouts) = atlases.get(&atlas.layout) {
                atlas.index = (atlas.index + 1) % layouts.textures.len();
            }
        }
    }
}
