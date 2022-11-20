use bevy::prelude::*;
use bevy_sprite3d::*;
use bevy_asset_loader::prelude::*;
use bevy::core_pipeline::clear_color::ClearColorConfig;
use rand::prelude::SliceRandom;



#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum GameState { Loading, Ready }

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16., columns = 30, rows = 35, padding_x = 10., padding_y = 10., offset_x = 5., offset_y = 5.))]
    #[asset(path = "dungeon/art/tileset_padded.png")]
    tileset: Handle<TextureAtlas>,
}

fn main() {

    App::new()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Ready)
                .with_collection::<ImageAssets>()
        )
        .add_state(GameState::Loading)
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(Sprite3dPlugin)
        .add_system_set( SystemSet::on_enter(GameState::Ready).with_system(setup) )
        .add_system_set( SystemSet::on_update(GameState::Ready).with_system(animate_camera) )
        .run();

}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn setup(
    mut commands: Commands, 
    images: Res<ImageAssets>,
    mut sprite_params: Sprite3dParams,
) {
    commands.spawn(Camera3dBundle {
        camera_3d: Camera3d {
            clear_color: ClearColorConfig::Custom(Color::rgb(0.0, 0.0, 0.0)),
            ..Default::default()
        },
        ..Default::default()
    });
    // // plane
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
    //     material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
    //     ..default()
    // });
    // // cube
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //     material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //     transform: Transform::from_xyz(0.0, 0.5, 0.0),
    //     ..default()
    // });
    // // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // ------------------ Tilemap for the floor ------------------

    // random floor tile
    let options_f = [(7,0), (7,0), (7,0), (9,1), (9,2), (9,3), (9,4)];
    let f = || { options_f.choose(&mut rand::thread_rng()).unwrap().clone() };

    let options_d = [(9,9), (9,10), (9,11)]; // random darker floor tile
    let d = || { options_d.choose(&mut rand::thread_rng()).unwrap().clone() };

    let options_l = [(7,5), (7,6), (7,7)]; // left wall tile
    let l = || { options_l.choose(&mut rand::thread_rng()).unwrap().clone() };
    let options_t = [(7,8), (7,9), (7,10)]; // top wall tile
    let t = || { options_t.choose(&mut rand::thread_rng()).unwrap().clone() };
    let options_b = [(7,11), (7,12), (7,13)]; // bottom wall tile
    let b = || { options_b.choose(&mut rand::thread_rng()).unwrap().clone() };
    let options_r = [(7,14), (7,15), (7,16)]; // right wall tile
    let r = || { options_r.choose(&mut rand::thread_rng()).unwrap().clone() };

    let tl = || { (7,1) }; // top left corner
    let tr = || { (7,2) }; // top right corner
    let bl = || { (7,3) }; // bottom left corner
    let br = || { (7,4) }; // bottom right corner

    let options_tb = [(7,21), (7,22)]; // top and bottom wall tile
    let tb = || { options_tb.choose(&mut rand::thread_rng()).unwrap().clone() };

    let map = vec![
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), tl(),  t(),   d(),   d(),   d(),   t(),   tr() ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   f(),   f(),   f(),   f(),   r()  ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), d(),   f(),   d(),   d(),   d(),   f(),   d()  ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), d(),   f(),   d(),   d(),   d(),   f(),   d()  ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), d(),   f(),   d(),   d(),   d(),   f(),   d()  ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   f(),   f(),   f(),   f(),   r()  ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), bl(),  b(), (8,21),  d(), (8,22),  b(),   br() ],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   r(),   (0,0), (0,0)],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), l(),   d(),   r(),   (0,0), (0,0)],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), tl(), (8,19), f(),  (8,20), tr(),  (0,0)],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   d(),   f(),   r(),   (0,0)],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   f(),   f(),   r(),   (0,0)],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   d(),   f(),   r(),   (0,0)],
        vec![(0,0), (0,0), (0,0), (0,0), (0,0), (0,0), l(),   f(),   f(),   f(),   r(),   (0,0)],
        vec![tl(),  t(),    tr(), (0,0), (0,0), (0,0), l(),   f(),   f(),   f(),   r(),   (0,0)],
        vec![l(),   f(),  (8,25),  tb(),  tb(),  tb(), (8,24),f(),   f(),   f(),   r(),   (0,0)],
        vec![bl(),  b(),    br(), (0,0), (0,0), (0,0), bl(),  b(),   b(),   b(),   br(),  (0,0)],
    ];




    for y in 0..map.len() {
        for x in 0..map[y].len() {
            let index = map[y][x].0 * 30 + map[y][x].1;
            let (x, y) = (x as f32 - 12.0 / 2.0, y as f32 - 17.0 / 2.0);
            if index == 0 { continue; }

            commands.spawn(AtlasSprite3d {
                    atlas: images.tileset.clone(),
                    pixels_per_metre: 16.,
                    partial_alpha: false,
                    unlit: false,
                    index: index as usize,
                    transform: Transform::from_xyz(x, 0.0, y).with_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0)),
                    ..default()
            }.bundle(&mut sprite_params));
        }
    }

    



    // -----------------------------------------------------------------------
}

const CAM_DISTANCE: f32 = 15.0;
const CAM_HEIGHT: f32 = 7.0;
const CAM_SPEED: f32 = 0.1;

fn animate_camera(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut transform = query.single_mut();
    let time = time.elapsed_seconds() * CAM_SPEED;
    transform.translation.x = time.sin() * CAM_DISTANCE;
    transform.translation.y = CAM_HEIGHT;
    transform.translation.z = time.cos() * CAM_DISTANCE;
    transform.look_at(Vec3::ZERO, Vec3::Y);
}


