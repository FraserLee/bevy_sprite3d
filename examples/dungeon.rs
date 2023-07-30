use bevy::asset::LoadState;
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasBundle;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::ScreenSpaceAmbientOcclusionBundle;
use bevy::render::camera::PerspectiveProjection;
use bevy::utils::Duration;
use bevy::{prelude::*, window::WindowResolution};

use bevy_sprite3d::*;

use rand::{prelude::SliceRandom, Rng};

#[derive(States, Hash, Clone, PartialEq, Eq, Debug, Default)]
enum GameState {
    #[default]
    Loading,
    Ready,
}

// #[derive(AssetCollection, Resource)]
// struct ImageAssets {
//     #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16.,
//             columns = 30, rows = 35, padding_x = 10., padding_y = 10.,
//             offset_x = 5., offset_y = 5.))]
//     #[asset(path = "dungeon/art/tileset_padded.png")]
//     tileset: Handle<TextureAtlas>,
// }
#[derive(Resource, Default)]
struct ImageAssets {
    image: Handle<Image>,
    tileset: Handle<TextureAtlas>,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(1080.0, 1080.0 * 3. / 4.),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Msaa::Off)
        .add_plugins(Sprite3dPlugin)
        .add_state::<GameState>()
        // initially load assets
        .add_systems(
            Startup,
            |asset_server: Res<AssetServer>,
             mut assets: ResMut<ImageAssets>,
             mut texture_atlases: ResMut<Assets<TextureAtlas>>| {
                assets.image = asset_server.load("dungeon/art/tileset_padded.png");

                assets.tileset = texture_atlases.add(TextureAtlas::from_grid(
                    assets.image.clone(),
                    Vec2::new(16.0, 16.0),
                    30,
                    35,
                    Some(Vec2::new(10., 10.)),
                    Some(Vec2::new(5., 5.)),
                ));
            },
        )
        // every frame check if assets are loaded. Once they are, we can proceed with setup.
        .add_systems(
            Update,
            (|asset_server: Res<AssetServer>,
              assets: Res<ImageAssets>,
              mut next_state: ResMut<NextState<GameState>>| {
                if asset_server.get_load_state(assets.image.clone()) == LoadState::Loaded {
                    next_state.set(GameState::Ready);
                }
            })
            .run_if(in_state(GameState::Loading)),
        )
        .add_systems(OnEnter(GameState::Ready), setup)
        .add_systems(OnEnter(GameState::Ready), spawn_sprites)
        .add_systems(Update, animate_camera.run_if(in_state(GameState::Ready)))
        .add_systems(Update, animate_sprites.run_if(in_state(GameState::Ready)))
        .add_systems(Update, face_camera.run_if(in_state(GameState::Ready)))
        .insert_resource(ImageAssets::default())
        .run();
}

#[derive(Component)]
struct FaceCamera; // tag entity to make it always face the camera

#[derive(Component)]
struct Animation {
    frames: Vec<usize>, // indices of all the frames in the animation
    current: usize,
    timer: Timer,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::WHITE.into()),
        transform: Transform::from_xyz(-0.9, 0.5, -3.1),
        ..default()
    });
    // sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(
            shape::Icosphere {
                radius: 0.6,
                subdivisions: 20,
            }
            .try_into()
            .unwrap(),
        ),
        material: materials.add(Color::WHITE.into()),
        transform: Transform::from_xyz(-0.9, 0.5, -4.2),
        ..default()
    });

    // camera
    commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                camera_3d: Camera3d {
                    clear_color: ClearColorConfig::Custom(Color::rgb(1.0, 1.0, 1.0) * 0.0),
                    ..default()
                },
                projection: bevy::prelude::Projection::Perspective(PerspectiveProjection {
                    fov: std::f32::consts::PI / 6.0,
                    ..default()
                }),
                ..default()
            },
            BloomSettings {
                intensity: 0.3,
                ..default()
            },
        ))
        .insert(ScreenSpaceAmbientOcclusionBundle::default())
        .insert(TemporalAntiAliasBundle::default());

    commands.spawn(Tonemapping::AcesFitted);
}

fn spawn_sprites(
    mut commands: Commands,
    images: Res<ImageAssets>,
    mut sprite_params: Sprite3dParams,
) {
    // ------------------ Tilemap for the floor ------------------

    // we first set up a few closures to help generate variations of tiles

    // random floor tile
    let options_f = [(7, 0), (7, 0), (7, 0), (9, 1), (9, 2), (9, 3), (9, 4)];
    let f = || options_f.choose(&mut rand::thread_rng()).unwrap().clone();

    let options_d = [(9, 9), (9, 10), (9, 11)]; // random darker floor tile
    let d = || options_d.choose(&mut rand::thread_rng()).unwrap().clone();

    let options_l = [(7, 5), (7, 6), (7, 7)]; // left wall tile
    let l = || options_l.choose(&mut rand::thread_rng()).unwrap().clone();
    let options_t = [(7, 8), (7, 9), (7, 10)]; // top wall tile
    let t = || options_t.choose(&mut rand::thread_rng()).unwrap().clone();
    let options_b = [(7, 11), (7, 12), (7, 13)]; // bottom wall tile
    let b = || options_b.choose(&mut rand::thread_rng()).unwrap().clone();
    let options_r = [(7, 14), (7, 15), (7, 16)]; // right wall tile
    let r = || options_r.choose(&mut rand::thread_rng()).unwrap().clone();

    let tl = || (7, 1); // top left corner
    let tr = || (7, 2); // top right corner
    let bl = || (7, 3); // bottom left corner
    let br = || (7, 4); // bottom right corner

    let options_tb = [(7, 21), (7, 22)]; // top and bottom wall tile
    let tb = || options_tb.choose(&mut rand::thread_rng()).unwrap().clone();

    // in reality, you'd probably want to import a map generated by an
    // external tool, or maybe proc-gen it yourself. For this example, a
    // 2d array should suffice.

    let mut map = vec![
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            tl(),
            t(),
            d(),
            d(),
            d(),
            t(),
            tr(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            f(),
            f(),
            f(),
            f(),
            r(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            d(),
            f(),
            d(),
            d(),
            d(),
            f(),
            d(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            d(),
            f(),
            d(),
            d(),
            d(),
            f(),
            d(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            d(),
            f(),
            d(),
            d(),
            d(),
            f(),
            d(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            f(),
            f(),
            f(),
            f(),
            r(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            bl(),
            b(),
            (8, 21),
            d(),
            (8, 22),
            b(),
            br(),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            r(),
            (0, 0),
            (0, 0),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            d(),
            r(),
            (0, 0),
            (0, 0),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            tl(),
            (8, 19),
            f(),
            (8, 20),
            tr(),
            (0, 0),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            d(),
            f(),
            r(),
            (0, 0),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            f(),
            f(),
            r(),
            (0, 0),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            d(),
            f(),
            r(),
            (0, 0),
        ],
        vec![
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            f(),
            f(),
            r(),
            (0, 0),
        ],
        vec![
            tl(),
            t(),
            tr(),
            (0, 0),
            (0, 0),
            (0, 0),
            l(),
            f(),
            f(),
            f(),
            r(),
            (0, 0),
        ],
        vec![
            l(),
            f(),
            (8, 25),
            tb(),
            tb(),
            tb(),
            (8, 24),
            f(),
            f(),
            f(),
            r(),
            (0, 0),
        ],
        vec![
            bl(),
            b(),
            br(),
            (0, 0),
            (0, 0),
            (0, 0),
            bl(),
            b(),
            b(),
            b(),
            br(),
            (0, 0),
        ],
    ];

    // add zero padding to the map
    map.insert(0, vec![(0, 0); map[0].len()]);
    map.push(vec![(0, 0); map[0].len()]);
    for row in map.iter_mut() {
        row.insert(0, (0, 0));
        row.push((0, 0));
    }

    // might be nice to add built-in support for sprite-merging for tilemaps...
    // though since all the meshes and materials are already cached and reused,
    // I wonder how much of a speedup that'd actually be. Food for thought.

    for y in 0..map.len() {
        for x in 0..map[y].len() {
            let index = map[y][x].0 * 30 + map[y][x].1;
            let (x, y) = (
                x as f32 - map[y].len() as f32 / 2.0,
                y as f32 - map.len() as f32 / 2.0,
            );
            if index == 0 {
                continue;
            }

            commands.spawn(
                AtlasSprite3d {
                    atlas: images.tileset.clone(),
                    pixels_per_meter: 16.,
                    index: index as usize,
                    double_sided: false,
                    transform: Transform::from_xyz(x, 0.0, y)
                        .with_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0)),
                    ..default()
                }
                .bundle(&mut sprite_params),
            );
        }
    }

    // --------------------------- add some walls -------------------------

    // first horizontally, then vertically, scan along the map. If we find
    // a point transitioning from (0,0) to something else, add a wall there.

    let mut rng = rand::thread_rng();

    // quick closure to get a random wall tile, avoiding staircases right next
    // to each other (since that can look weird)
    let mut time_since_staircase = 0;
    let mut wall_index = || {
        if time_since_staircase > 3 && rng.gen_bool(0.075) {
            time_since_staircase = 0;
            if rng.gen_bool(0.5) {
                7
            } else {
                8
            }
        } else {
            time_since_staircase += 1;
            if rng.gen_bool(0.6) {
                1
            } else {
                rng.gen_range(2..=4)
            }
        }
    };

    for y in 1..(map.len() - 1) {
        for x in 0..(map[y].len() - 1) {
            if (map[y][x] != (0, 0)) ^ (map[y][x + 1] == (0, 0)) {
                continue;
            }
            let dir = if map[y][x] == (0, 0) { 1.0 } else { -1.0 };

            let mut tile_x = wall_index();

            if map[y][x] == (0, 0) {
                // literal corner cases. hah.
                if map[y + 1][x + 1] == (0, 0) {
                    tile_x = 0;
                }
                if map[y - 1][x + 1] == (0, 0) {
                    tile_x = 5;
                }
            } else {
                if map[y - 1][x] == (0, 0) {
                    tile_x = 0;
                }
                if map[y + 1][x] == (0, 0) {
                    tile_x = 5;
                }
            }

            let (x, y) = (
                x as f32 - map[y].len() as f32 / 2.0,
                y as f32 - map.len() as f32 / 2.0,
            );

            for i in [0, 1] {
                // add bottom and top piece
                commands.spawn(
                    AtlasSprite3d {
                        atlas: images.tileset.clone(),
                        pixels_per_meter: 16.,
                        index: (tile_x + (5 - i) * 30) as usize,
                        double_sided: false,
                        transform: Transform::from_xyz(x + 0.5, i as f32 + 0.499, y)
                            .with_rotation(Quat::from_rotation_y(dir * std::f32::consts::PI / 2.0)),
                        ..default()
                    }
                    .bundle(&mut sprite_params),
                );
            }
        }
    }

    // same thing again, but for the vertical walls
    for x in 1..(map[0].len() - 1) {
        for y in 0..(map.len() - 1) {
            if (map[y][x] != (0, 0)) ^ (map[y + 1][x] == (0, 0)) {
                continue;
            }
            let dir = if map[y][x] == (0, 0) { 1.0 } else { -1.0 };

            let mut tile_x = wall_index();

            if map[y][x] == (0, 0) {
                if map[y + 1][x - 1] == (0, 0) {
                    tile_x = 0;
                }
                if map[y + 1][x + 1] == (0, 0) {
                    tile_x = 5;
                }
            } else {
                if map[y][x + 1] == (0, 0) {
                    tile_x = 0;
                }
                if map[y][x - 1] == (0, 0) {
                    tile_x = 5;
                }
            }

            let (x, y) = (
                x as f32 - map[y].len() as f32 / 2.0,
                y as f32 - map.len() as f32 / 2.0,
            );

            for i in [0, 1] {
                // add bottom and top piece
                commands.spawn(
                    AtlasSprite3d {
                        atlas: images.tileset.clone(),
                        pixels_per_meter: 16.,
                        index: (tile_x + (5 - i) * 30) as usize,
                        double_sided: false,
                        transform: Transform::from_xyz(x, i as f32 + 0.499, y + 0.5).with_rotation(
                            Quat::from_rotation_y((dir - 1.0) * std::f32::consts::PI / 2.0),
                        ),
                        ..default()
                    }
                    .bundle(&mut sprite_params),
                );
            }
        }
    }

    // --------------------- characters, enemies, props ---------------------

    let mut entity = |(x, y), tile_x, tile_y, height, frames| {
        let mut timer = Timer::from_seconds(0.4, TimerMode::Repeating);
        timer.set_elapsed(Duration::from_secs_f32(rng.gen_range(0.0..0.4)));

        for i in 0usize..height {
            let mut c = commands.spawn((
                AtlasSprite3d {
                    atlas: images.tileset.clone(),
                    pixels_per_meter: 16.,
                    index: (tile_x + (tile_y - i) * 30) as usize,
                    transform: Transform::from_xyz(x as f32, i as f32 + 0.498, y),
                    ..default()
                }
                .bundle(&mut sprite_params),
                FaceCamera {},
            ));

            if frames > 1 {
                c.insert(Animation {
                    frames: (0..frames)
                        .map(|j| j + tile_x + (tile_y - i) * 30 as usize)
                        .collect(),
                    current: 0,
                    timer: timer.clone(),
                });
            }
        }
    };

    // 3 humans
    entity((4.5, -4.0), 8, 27, 2, 2);
    entity((1.5, -7.0), 4, 27, 2, 2);
    entity((0.5, 2.0), 6, 27, 2, 2);

    // 5 containers
    entity((3.5, 1.0), 0, 19, 1, 1);
    entity((4.0, 6.0), 1, 19, 1, 1);
    entity((0.0, 5.0), 4, 19, 1, 1);
    entity((-4.0, 5.5), 5, 19, 1, 1);
    entity((-0.5, -8.5), 2, 19, 1, 1);

    // ikea chair
    entity((4.2, -8.), 13, 16, 2, 1);

    // fire
    commands.spawn((
        AtlasSprite3d {
            atlas: images.tileset.clone(),
            pixels_per_meter: 16.,
            index: 30 * 32 + 14,
            transform: Transform::from_xyz(2.0, 0.5, -5.5),
            emissive: Color::rgb(1.0, 0.5, 0.0) * 10.0,
            unlit: true,
            ..default()
        }
        .bundle(&mut sprite_params),
        Animation {
            frames: vec![30 * 32 + 14, 30 * 32 + 15, 30 * 32 + 16],
            current: 0,
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        },
        FaceCamera {},
    ));
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 300.0,
            color: Color::rgb(1.0, 231. / 255., 221. / 255.),
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(2.0, 1.8, -5.5),
        ..default()
    });

    // glowy book
    commands.spawn((
        AtlasSprite3d {
            atlas: images.tileset.clone(),
            pixels_per_meter: 16.,
            index: 22 * 30 + 22,
            transform: Transform::from_xyz(-5., 0.7, 6.5),
            emissive: Color::rgb(165. / 255., 1.0, 160. / 255.),
            unlit: true,
            ..default()
        }
        .bundle(&mut sprite_params),
        FaceCamera {},
    ));
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 100.0,
            color: Color::rgb(91. / 255., 1.0, 92. / 255.),
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-5., 1.1, 6.5),
        ..default()
    });
}

// parameters for how the camera orbits the area
const CAM_DISTANCE: f32 = 25.0;
const CAM_HEIGHT: f32 = 16.0;
const CAM_SPEED: f32 = -0.1;

// camera will always orbit 0,0,0, but can look somewhere slightly different
const CAM_TARGET_X: f32 = 2.0;
const CAM_TARGET_Z: f32 = -5.5;

const CAM_T_OFFSET: f32 = -0.4;

fn animate_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera>>) {
    let mut transform = query.single_mut();
    let time = std::f32::consts::PI - time.elapsed_seconds() * CAM_SPEED + CAM_T_OFFSET;
    transform.translation.x = time.sin() * CAM_DISTANCE;
    transform.translation.y = CAM_HEIGHT;
    transform.translation.z = time.cos() * CAM_DISTANCE;
    transform.look_at(Vec3::new(CAM_TARGET_X, 0.0, CAM_TARGET_Z), Vec3::Y);
}

fn animate_sprites(
    time: Res<Time>,
    mut query: Query<(&mut Animation, &mut AtlasSprite3dComponent)>,
) {
    for (mut animation, mut sprite) in query.iter_mut() {
        animation.timer.tick(time.delta());
        if animation.timer.just_finished() {
            sprite.index = animation.frames[animation.current];
            animation.current += 1;
            animation.current %= animation.frames.len();
        }
    }
}

fn face_camera(
    cam_query: Query<&Transform, With<Camera>>,
    mut query: Query<&mut Transform, (With<FaceCamera>, Without<Camera>)>,
) {
    let cam_transform = cam_query.single();
    for mut transform in query.iter_mut() {
        let mut delta = cam_transform.translation - transform.translation;
        delta.y = 0.0;
        delta += transform.translation;
        transform.look_at(delta, Vec3::Y);
    }
}
