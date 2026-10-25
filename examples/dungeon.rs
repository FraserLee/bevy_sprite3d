use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::visibility::VisibilitySystems;
use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::material::specialize::SpecializedMeshPipelineError;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{ContactShadows, MaterialPipeline, MaterialPipelineKey, ScreenSpaceAmbientOcclusion};
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::post_process::bloom::Bloom;
use bevy::render::render_resource::{AsBindGroup, CompareFunction, RenderPipelineDescriptor};
use bevy::shader::ShaderRef;
use bevy::{prelude::*, window::WindowResolution};
use std::time::Duration;

use bevy_sprite3d::prelude::*;
use bevy_sprite3d::{SPRITE3D_ATTRIBUTE_RECT, SPRITE3D_ATTRIBUTE_UV_RECT};

use rand::{prelude::SliceRandom, Rng};


#[rustfmt::skip]
fn main()
{
    App::new()
        .add_plugins(DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some( Window{
                    resolution: WindowResolution::new(1080, (1080.0 * 3./4.) as u32),
                    ..default()
                }), ..default()
            }))
        .add_plugins((Sprite3dPlugin,
                      // hovering a character or prop highlights and outlines it
                      MeshPickingPlugin, MaterialPlugin::<OutlineMaterial>::default()))
        .add_systems(Startup, (setup, spawn_sprites))
        .add_systems(Update, (animate_camera, animate_sprites, face_camera))
        // copy the meshes bevy_sprite3d assigned this frame onto the outlines,
        // before the visibility pass computes their bounds.
        .add_systems(PostUpdate, sync_outlines.after(Sprite3dSystems)
                                              .before(VisibilitySystems::CalculateBounds))
        .run();
}

// Draws a one-texel outline around a sprite, over top of everything else. Goes on a child of the
// sprite that borrows the sprite's mesh.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct OutlineMaterial
{
    #[texture(0, visibility(vertex, fragment))]
    #[sampler(1)]
    image: Handle<Image>,
    #[uniform(2)]
    color: LinearRgba,
    /// x: grow upwards (0 or 1), y: grow downwards. Where a sprite continues
    /// into the piece above or below, that side isn't grown, so no outline is
    /// drawn across the seam.
    #[uniform(3)]
    grow: Vec4,
}

#[rustfmt::skip]
impl Material for OutlineMaterial
{
    fn vertex_shader() -> ShaderRef { "shaders/outline.wesl".into() }
    fn fragment_shader() -> ShaderRef { "shaders/outline.wesl".into() }
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Blend } // drawn after the opaque pass

    fn specialize(_: &MaterialPipeline,
                  descriptor: &mut RenderPipelineDescriptor,
                  layout: &MeshVertexBufferLayoutRef,
                  _: MaterialPipelineKey<Self>)
                  -> Result<(), SpecializedMeshPipelineError>
    {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            SPRITE3D_ATTRIBUTE_RECT.at_shader_location(2),
            SPRITE3D_ATTRIBUTE_UV_RECT.at_shader_location(3),
        ])?];
        // ignore depth so the outline is visible through walls
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

#[derive(Component)]
struct Outline;

fn sync_outlines(mut outlines: Query<(&ChildOf, &mut Mesh3d), With<Outline>>,
                 sprites: Query<&Mesh3d, Without<Outline>>)
{
    for (parent, mut mesh) in &mut outlines {
        mesh.set_if_neq(sprites.get(parent.parent()).unwrap().clone());
    }
}

#[derive(Component)]
struct FaceCamera; // tag entity to make it always face the camera

#[derive(Component)]
struct Animation
{
    frames:  Vec<usize>, // indices of all the frames in the animation
    current: usize,
    timer:   Timer,
}

#[rustfmt::skip]
fn setup(mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>)
{
    // cube
    commands.spawn((Mesh3d(meshes.add(Mesh::from(Cuboid::from_size(Vec3::splat(1.0))))),
                    MeshMaterial3d(materials.add(Color::WHITE)),
                    Transform::from_xyz(-0.9, 0.5, -3.1)));
    // sphere
    commands.spawn((Mesh3d(meshes.add(Sphere::new(0.6))),
                    MeshMaterial3d(materials.add(Color::WHITE)),
                    Transform::from_xyz(-0.9, 0.5, -4.2)));

    // camera
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        Projection::Perspective(PerspectiveProjection { fov: std::f32::consts::PI / 6.0, ..default() }),
        Hdr,
        Msaa::Off,
        Bloom { intensity: 0.3, ..default() },
        ScreenSpaceAmbientOcclusion::default(),
        ContactShadows::default(),
        TemporalAntiAliasing::default(),
        Tonemapping::AcesFitted,
    ));
}

#[rustfmt::skip]
fn spawn_sprites(mut commands: Commands,
                 asset_server: Res<AssetServer>,
                 mut layouts: ResMut<Assets<TextureAtlasLayout>>,
                 mut outlines: ResMut<Assets<OutlineMaterial>>)
{
    let image = asset_server.load("dungeon/tileset_padded.png");
    // one outline material per combination of open sides
    let outline = |up: bool, down: bool| OutlineMaterial {
        image: image.clone(),
        color: LinearRgba::rgb(4.0, 3.5, 1.0),
        grow:  Vec4::new(up as u32 as f32, down as u32 as f32, 0., 0.),
    };
    let outline_both = outlines.add(outline(true, true));
    let outline_up = outlines.add(outline(true, false));
    let outline_down = outlines.add(outline(false, true));
    let layout = layouts.add(TextureAtlasLayout::from_grid(UVec2::new(16, 16),
                                                           30,
                                                           35,
                                                           Some(UVec2::new(10, 10)),
                                                           Some(UVec2::new(5, 5))));

    // Mask alpha keeps the sprites out of the (sorted) transparent pass
    let sprite = |index: usize| Sprite {
        image: image.clone(),
        texture_atlas: Some(TextureAtlas { layout: layout.clone(), index }),
        alpha_mode: SpriteAlphaMode::Mask(0.5),
        ..default()
    };
    let sprite3d = Sprite3d { pixels_per_metre: 16., double_sided: false, ..default() };

    // ------------------ Tilemap for the floor ------------------

    // we first set up a few closures to help generate variations of tiles

    // random floor tile
    let options_f = [(7, 0), (7, 0), (7, 0), (9, 1), (9, 2), (9, 3), (9, 4)];
    let f = || *options_f.choose(&mut rand::thread_rng()).unwrap();

    let options_d = [(9, 9), (9, 10), (9, 11)]; // random darker floor tile
    let d = || *options_d.choose(&mut rand::thread_rng()).unwrap();

    let options_l = [(7, 5), (7, 6), (7, 7)]; // left wall tile
    let l = || *options_l.choose(&mut rand::thread_rng()).unwrap();
    let options_t = [(7, 8), (7, 9), (7, 10)]; // top wall tile
    let t = || *options_t.choose(&mut rand::thread_rng()).unwrap();
    let options_b = [(7, 11), (7, 12), (7, 13)]; // bottom wall tile
    let b = || *options_b.choose(&mut rand::thread_rng()).unwrap();
    let options_r = [(7, 14), (7, 15), (7, 16)]; // right wall tile
    let r = || *options_r.choose(&mut rand::thread_rng()).unwrap();

    let tl = || (7, 1); // top left corner
    let tr = || (7, 2); // top right corner
    let bl = || (7, 3); // bottom left corner
    let br = || (7, 4); // bottom right corner

    let options_tb = [(7, 21), (7, 22)]; // top and bottom wall tile
    let tb = || *options_tb.choose(&mut rand::thread_rng()).unwrap();

    // in reality, you'd probably want to import a map generated by an
    // external tool, or maybe proc-gen it yourself. For this example, a
    // 2d array should suffice.

    #[rustfmt::skip]
    let mut map = vec![
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
            let x = x as f32 - map[y].len() as f32 / 2.0;
            let y = y as f32 - map.len() as f32 / 2.0;
            if index == 0 {
                continue;
            }

            commands.spawn((
                sprite(index),
                sprite3d.clone(),
                Transform::from_xyz(x, 0.0, y)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0))
            ));
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
            if rng.gen_bool(0.5) { 7 } else { 8 }
        } else {
            time_since_staircase += 1;
            if rng.gen_bool(0.6) { 1 } else { rng.gen_range(2..=4) }
        }
    };

    for y in 1..(map.len() - 1) {
        for x in 0..(map[y].len() - 1) {
            if (map[y][x] != (0, 0)) ^ (map[y][x + 1] == (0, 0)) {
                continue;
            }
            let dir = if map[y][x] == (0, 0) { 1.0 } else { -1.0 };

            let mut tile_x = wall_index();

            if map[y][x] == (0, 0) { // literal corner cases. hah.
                if map[y + 1][x + 1] == (0, 0) { tile_x = 0; }
                if map[y - 1][x + 1] == (0, 0) { tile_x = 5; }
            } else {
                if map[y - 1][x] == (0, 0) { tile_x = 0; }
                if map[y + 1][x] == (0, 0) { tile_x = 5; }
            }

            let x = x as f32 - map[y].len() as f32 / 2.0;
            let y = y as f32 - map.len() as f32 / 2.0;

            for i in [0,1] { // add bottom and top piece
                commands.spawn((
                    sprite(tile_x + (5 - i) * 30),
                    sprite3d.clone(),
                    Transform::from_xyz(x+0.5, i as f32 + 0.499, y)
                        .with_rotation(Quat::from_rotation_y(dir * std::f32::consts::PI / 2.0))
                ));
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
                if map[y + 1][x - 1] == (0, 0) { tile_x = 0; }
                if map[y + 1][x + 1] == (0, 0) { tile_x = 5; }
            } else {
                if map[y][x + 1] == (0, 0) { tile_x = 0; }
                if map[y][x - 1] == (0, 0) { tile_x = 5; }
            }

            let x = x as f32 - map[y].len() as f32 / 2.0;
            let y = y as f32 - map.len() as f32 / 2.0;

            for i in [0, 1] { // add bottom and top piece
                commands.spawn((
                    sprite(tile_x + (5 - i) * 30),
                    sprite3d.clone(),
                    Transform::from_xyz(x, i as f32 + 0.499, y + 0.5)
                        .with_rotation(Quat::from_rotation_y((dir - 1.0) * std::f32::consts::PI / 2.0)),
                ));
            }
        }
    }

    // --------------------- characters, enemies, props ---------------------

    // a parent (faces the camera, hoverable) with a child sprite per tile of
    // height, each with a hidden outline child.
    let mut prop = |(x, y, z), tile_x: usize, tile_y: usize, height: usize, frames: usize,
                    frame_secs: f32, emissive: LinearRgba| {
        let mut timer = Timer::from_seconds(frame_secs, TimerMode::Repeating);
        timer.set_elapsed(Duration::from_secs_f32(rng.gen_range(0.0..frame_secs)));

        commands.spawn((Transform::from_xyz(x, y, z), Visibility::default(), FaceCamera))
                .with_children(|parent| {
                    for i in 0..height {
                        let index = tile_x + (tile_y - i) * 30;
                        let mut c = parent.spawn((sprite(index),
                                                  Sprite3d { pixels_per_metre: 16.,
                                                             emissive,
                                                             unlit: emissive != LinearRgba::BLACK,
                                                             ..default() },
                                                  Transform::from_xyz(0.0, i as f32, 0.0)));
                        if frames > 1 {
                            c.insert(Animation { frames:  (index..index + frames).collect(),
                                                 current: 0,
                                                 timer:   timer.clone() });
                        }
                        let outline = match (i == height - 1, i == 0) {
                            (true, true) => &outline_both,
                            (true, false) => &outline_up,
                            _ => &outline_down,
                        };
                        c.with_child((Outline, Mesh3d::default(), MeshMaterial3d(outline.clone()),
                                      Visibility::Hidden, Pickable::IGNORE));
                    }
                })
                .observe(highlight::<PointerEnter>(true))
                .observe(highlight::<PointerLeave>(false));
    };

    let mut entity = |(x, z), tile_x, tile_y, height, frames| prop((x, 0.498, z), tile_x, tile_y, height, frames, 0.4, LinearRgba::BLACK);

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
    prop((2.0, 0.5, -5.5), 14, 32, 1, 3, 0.2, LinearRgba::rgb(1.0, 0.5, 0.0) * 10.0);

    // glowy book
    prop((-5., 0.7, 6.5), 22, 22, 1, 1, 0.4, LinearRgba::rgb(165./255., 1.0, 160./255.));

    // their lights
    commands.spawn((
        PointLight {
            intensity: 500_000.0,
            color: Color::srgb(1.0, 231./255., 221./255.),
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 1.8, -5.5),
    ));
    commands.spawn((
        PointLight {
            intensity: 70_000.0,
            color: Color::srgb(91./255., 1.0, 92./255.),
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-5., 1.1, 6.5),
    ));
}


// highlights every sprite under the hovered entity. Enter / Leave rather than
// Over / Out, so moving between the pieces of one prop doesn't un-highlight it.
#[rustfmt::skip] #[allow(clippy::type_complexity)]
fn highlight<E: EntityEvent>(on: bool)
    -> impl Fn(On<E>, Query<&Children>, Query<(&mut Sprite, &mut Sprite3d)>, Query<&mut Visibility, With<Outline>>)
{
    move |event, children, mut sprites, mut outlines| {
        for e in children.iter_descendants(event.event_target()) {
            if let Ok((mut sprite, mut sprite3d)) = sprites.get_mut(e) {
                sprite.color = if on { Color::linear_rgb(1.5, 1.5, 1.2) } else { Color::WHITE };
                sprite3d.unlit = on || sprite3d.emissive != LinearRgba::BLACK;
            }
            if let Ok(mut visibility) = outlines.get_mut(e) {
                *visibility = if on { Visibility::Inherited } else { Visibility::Hidden };
            }
        }
    }
}

// parameters for how the camera orbits the area
const CAM_DISTANCE: f32 = 25.0;
const CAM_HEIGHT: f32 = 16.0;
const CAM_SPEED: f32 = -0.1;

// camera will always orbit 0,0,0, but can look somewhere slightly different
const CAM_TARGET_X: f32 = 2.0;
const CAM_TARGET_Z: f32 = -5.5;

const CAM_T_OFFSET: f32 = -0.4;

fn animate_camera(time: Res<Time>, mut transform: Single<&mut Transform, With<Camera>>)
{
    let time = std::f32::consts::PI - time.elapsed_secs() * CAM_SPEED + CAM_T_OFFSET;
    transform.translation.x = time.sin() * CAM_DISTANCE;
    transform.translation.y = CAM_HEIGHT;
    transform.translation.z = time.cos() * CAM_DISTANCE;
    transform.look_at(Vec3::new(CAM_TARGET_X, 0.0, CAM_TARGET_Z), Vec3::Y);
}


fn animate_sprites(time: Res<Time>, mut query: Query<(&mut Animation, &mut Sprite)>)
{
    for (mut animation, mut sprite) in query.iter_mut() {
        animation.timer.tick(time.delta());
        if animation.timer.just_finished() {
            let atlas = sprite.texture_atlas.as_mut().unwrap();
            atlas.index = animation.frames[animation.current];
            animation.current += 1;
            animation.current %= animation.frames.len();
        }
    }
}


#[rustfmt::skip]
fn face_camera(
    cam_transform: Single<&Transform, With<Camera>>,
    mut query: Query<&mut Transform, (With<FaceCamera>, Without<Camera>)>
) {
    for mut transform in query.iter_mut() {
        let mut delta = cam_transform.translation - transform.translation;
        delta.y = 0.0;
        delta += transform.translation;
        transform.look_at(delta, Vec3::Y);
    }
}
