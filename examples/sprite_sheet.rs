use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(Sprite3dPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_sprite)
        .run();

}

fn setup(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut billboards: ResMut<Assets<Billboard>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("gabe-idle-run.png");
    let layout = texture_atlases.add(
        TextureAtlasLayout::from_grid(UVec2::new(24, 24), 7, 1, None, None)
    );

    let billboard =
        Billboard::with_texture_atlas(image.clone(), layout.clone(), 32., None, false);
    let texture_atlas = TextureAtlas {
        layout,
        index: 3,
    };

    let material = StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..bevy_sprite3d::utils::material()
    };

    // spawn the camera
    commands.spawn(Camera3d::default()).insert(Transform::from_xyz(0., 0., 5.));

    // spawn an animated 3D sprite
    commands.spawn((
        Sprite3d::from(texture_atlas),
        Sprite3dBillboard::new(billboards.add(billboard)),
        MeshMaterial3d(materials.add(material)),
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));
}


fn animate_sprite(
    time: Res<Time>,
    atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut query: Query<(&mut AnimationTimer, &mut Sprite3d)>,
) {
    for (mut timer, mut sprite_3d) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            let atlas = sprite_3d.texture_atlas.as_mut().unwrap();
            let length = atlas_layouts.get(&atlas.layout).unwrap().textures.len();
            atlas.index = (atlas.index + 1) % length;
        }
    }
}
