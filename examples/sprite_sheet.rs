use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn main()
{
    App::new().add_plugins((DefaultPlugins.set(ImagePlugin::default_nearest()), Sprite3dPlugin))
              .add_systems(Startup, setup)
              .add_systems(Update, animate_sprite) // every frame, animate the sprite
              .run();
}

#[rustfmt::skip]
fn setup(mut commands: Commands,
         asset_server: Res<AssetServer>,
         mut layouts: ResMut<Assets<TextureAtlasLayout>>)
{
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0., 5.)));

    // -------------------- Spawn a 3D atlas sprite --------------------------

    let texture_atlas = TextureAtlas {
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(24, 24), // tile_size
            7,                  // columns
            1,                  // rows
            None,               // padding
            None,               // offset
        )),
        index: 3,
    };

    // the sprite's mesh is built as soon as the image finishes loading.
    commands.spawn((
        Sprite { image: asset_server.load("gabe-idle-run.png"), texture_atlas: Some(texture_atlas), ..default() },
        Sprite3d {
            pixels_per_metre: 32.,
            unlit: true,
            ..default()
        },
        // Anchor::CENTER, // default pivot
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    // -----------------------------------------------------------------------
}


fn animate_sprite(time: Res<Time>,
                  atlases: Res<Assets<TextureAtlasLayout>>,
                  mut query: Query<(&mut AnimationTimer, &mut Sprite)>)
{
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
