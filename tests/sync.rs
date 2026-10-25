use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::math::Vec3A;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_sprite3d::prelude::*;
use bevy_sprite3d::{Sprite3dCaches, SPRITE3D_ATTRIBUTE_RECT, SPRITE3D_ATTRIBUTE_UV_RECT};

fn app() -> App
{
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), Sprite3dPlugin));
    app.init_asset::<Image>()
       .init_asset::<Mesh>()
       .init_asset::<StandardMaterial>()
       .init_asset::<TextureAtlasLayout>();
    app
}

fn image(w: u32, h: u32) -> Image
{
    Image::new_fill(Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    &[0, 0, 0, 0],
                    TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::default())
}

fn grid(app: &mut App, tile: UVec2, columns: u32, rows: u32) -> Handle<TextureAtlasLayout>
{
    app.world_mut()
       .resource_mut::<Assets<TextureAtlasLayout>>()
       .add(TextureAtlasLayout::from_grid(tile, columns, rows, None, None))
}

fn mesh(app: &App, e: Entity) -> Mesh3d
{
    app.world().get::<Mesh3d>(e).unwrap().clone()
}

// world-space (width, height) of the entity's mesh
fn size(app: &App, e: Entity) -> Vec2
{
    let meshes = app.world().resource::<Assets<Mesh>>();
    let aabb = meshes.get(&mesh(app, e).0).unwrap().get_aabb().unwrap();
    (aabb.half_extents * 2.0).truncate()
}

fn set_atlas(app: &mut App, e: Entity, atlas: Option<TextureAtlas>)
{
    app.world_mut().get_mut::<Sprite>(e).unwrap().texture_atlas = atlas;
}

#[test]
fn atlas_changes_update_mesh()
{
    let mut app = app();
    let img = app.world_mut().resource_mut::<Assets<Image>>().add(image(64, 32));
    let a = grid(&mut app, UVec2::new(32, 32), 2, 1);
    let b = grid(&mut app, UVec2::new(16, 32), 4, 1);

    let e = app.world_mut()
               .spawn((Sprite3d::default(),
                       Sprite { image: img,
                                texture_atlas: Some(TextureAtlas { layout: a.clone(), index: 0 }),
                                ..default() }))
               .id();
    app.update();
    let a0 = mesh(&app, e);
    assert_ne!(a0, Mesh3d::default());
    assert_eq!(size(&app, e), Vec2::new(0.32, 0.32));

    // another frame of the same atlas
    set_atlas(&mut app, e, Some(TextureAtlas { layout: a.clone(), index: 1 }));
    app.update();
    let a1 = mesh(&app, e);
    assert_ne!(a1, a0);
    assert_eq!(size(&app, e), Vec2::new(0.32, 0.32));

    // a different atlas layout
    set_atlas(&mut app, e, Some(TextureAtlas { layout: b, index: 0 }));
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.16, 0.32));

    // no atlas: the whole image
    set_atlas(&mut app, e, None);
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.64, 0.32));

    // meshes are shared through the cache
    set_atlas(&mut app, e, Some(TextureAtlas { layout: a, index: 0 }));
    app.update();
    assert_eq!(mesh(&app, e), a0);
}

#[test]
fn spawn_before_image_loads()
{
    let mut app = app();
    let img = app.world().resource::<Assets<Image>>().reserve_handle();

    let e = app.world_mut()
               .spawn((Sprite3d::default(), Sprite { image: img.clone(), ..default() }))
               .id();
    app.update();
    assert_eq!(mesh(&app, e), Mesh3d::default());

    app.world_mut().resource_mut::<Assets<Image>>().insert(&img, image(50, 100)).unwrap();
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.5, 1.0));
}

#[test]
fn modifying_assets_in_place_updates_mesh()
{
    let mut app = app();
    let img = app.world_mut().resource_mut::<Assets<Image>>().add(image(64, 32));
    let layout = grid(&mut app, UVec2::new(32, 32), 2, 1);

    let e = app.world_mut()
               .spawn((Sprite3d::default(), Sprite { image: img.clone(), ..default() }))
               .id();
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.64, 0.32));

    // replace the image with one of a different size
    app.world_mut().resource_mut::<Assets<Image>>().insert(&img, image(32, 32)).unwrap();
    app.update();
    app.update(); // (modified-asset events are read on the following frame, see comments in lib.rs)
    assert_eq!(size(&app, e), Vec2::new(0.32, 0.32));

    set_atlas(&mut app, e, Some(TextureAtlas { layout: layout.clone(), index: 0 }));
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.32, 0.32));

    // shrink the atlas frame in place
    app.world_mut()
       .resource_mut::<Assets<TextureAtlasLayout>>()
       .get_mut(&layout)
       .unwrap()
       .textures[0] = URect::new(0, 0, 8, 16);
    app.update();
    app.update(); // <- again, double-update needed
    assert_eq!(size(&app, e), Vec2::new(0.08, 0.16));
}


fn material(app: &App, e: Entity) -> StandardMaterial
{
    let handle = app.world().get::<MeshMaterial3d<StandardMaterial>>(e).unwrap();
    app.world().resource::<Assets<StandardMaterial>>().get(&handle.0).unwrap().clone()
}

#[test]
fn anchor_offsets_mesh_and_bounds()
{
    let mut app = app();
    let img = app.world_mut().resource_mut::<Assets<Image>>().add(image(100, 50));

    let e = app.world_mut()
               .spawn((Sprite3d::default(), Sprite { image: img, ..default() }, Anchor::BOTTOM_LEFT))
               .id();
    app.update();

    // the quad's bottom left corner sits at the origin
    let expected = Aabb { center: Vec3A::new(0.5, 0.25, 0.), half_extents: Vec3A::new(0.5, 0.25, 0.) };
    let meshes = app.world().resource::<Assets<Mesh>>();
    assert_eq!(meshes.get(&mesh(&app, e).0).unwrap().get_aabb().unwrap(), expected);
    assert_eq!(*app.world().get::<Aabb>(e).unwrap(), expected);
    let rects = meshes.get(&mesh(&app, e).0).unwrap().attribute(SPRITE3D_ATTRIBUTE_RECT).unwrap();
    let VertexAttributeValues::Float32x4(rects) = rects else { panic!() };
    assert!(rects.iter().all(|r| *r == [0.0, 0.0, 1.0, 0.5]));

    // changing the anchor re-syncs
    *app.world_mut().get_mut::<Anchor>(e).unwrap() = Anchor::CENTER;
    app.update();
    assert_eq!(app.world().get::<Aabb>(e).unwrap().center, Vec3A::ZERO);
}

#[test]
fn custom_size_and_rect()
{
    let mut app = app();
    let img = app.world_mut().resource_mut::<Assets<Image>>().add(image(64, 32));
    let layout = grid(&mut app, UVec2::new(32, 32), 2, 1);

    let e = app.world_mut()
               .spawn((Sprite3d::default(),
                       Sprite { image: img, rect: Some(Rect::new(0., 0., 16., 8.)), ..default() }))
               .id();
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.16, 0.08));

    // rect is relative to the atlas frame
    set_atlas(&mut app, e, Some(TextureAtlas { layout, index: 1 }));
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.16, 0.08));
    let meshes = app.world().resource::<Assets<Mesh>>();
    let uvs = meshes.get(&mesh(&app, e).0).unwrap().attribute(Mesh::ATTRIBUTE_UV_0).unwrap();
    let VertexAttributeValues::Float32x2(uvs) = uvs else { panic!() };
    assert_eq!(uvs[2], [0.5, 0.0]); // top left vertex: left edge of frame 1

    // custom_size overrides the drawn region's size
    app.world_mut().get_mut::<Sprite>(e).unwrap().custom_size = Some(Vec2::new(10., 20.));
    app.update();
    assert_eq!(size(&app, e), Vec2::new(0.1, 0.2));

    // rects a texel apart on a wide atlas shouldn't reuse the same mesh
    let wide = app.world_mut().resource_mut::<Assets<Image>>().add(image(4096, 8));
    let spawn = |app: &mut App, x: f32| {
        let rect = Some(Rect::new(x, 0., x + 8., 8.));
        app.world_mut().spawn((Sprite3d::default(), Sprite { image: wide.clone(), rect, ..default() })).id()
    };
    let (a, b) = (spawn(&mut app, 2.), spawn(&mut app, 3.));
    app.update();
    assert_ne!(mesh(&app, a), mesh(&app, b));
}

#[test]
fn flips_stay_within_atlas_frame()
{
    let mut app = app();
    let img = app.world_mut().resource_mut::<Assets<Image>>().add(image(64, 32));
    let layout = grid(&mut app, UVec2::new(32, 32), 2, 1);

    let e = app.world_mut()
               .spawn((Sprite3d::default(),
                       Sprite { image: img,
                                texture_atlas: Some(TextureAtlas { layout, index: 1 }),
                                flip_x: true,
                                ..default() }))
               .id();
    app.update();

    let meshes = app.world().resource::<Assets<Mesh>>();
    let uvs = meshes.get(&mesh(&app, e).0).unwrap().attribute(Mesh::ATTRIBUTE_UV_0).unwrap();
    let VertexAttributeValues::Float32x2(uvs) = uvs else { panic!() };
    // left vertices sample the right edge of frame 1 and vice versa
    assert_eq!(uvs[0], [1.0, 1.0]);
    assert_eq!(uvs[1], [0.5, 1.0]);

    // but the uv rect handed to shaders is frame 1 as it lies in the image
    let rects = meshes.get(&mesh(&app, e).0).unwrap().attribute(SPRITE3D_ATTRIBUTE_UV_RECT).unwrap();
    let VertexAttributeValues::Float32x4(rects) = rects else { panic!() };
    assert!(rects.iter().all(|r| *r == [0.5, 0.0, 1.0, 1.0]));
}

#[test]
fn sprite_fields_drive_material()
{
    let mut app = app();
    let img = app.world_mut().resource_mut::<Assets<Image>>().add(image(8, 8));

    let spawn = |app: &mut App, color, alpha_mode| {
        app.world_mut()
           .spawn((Sprite3d::default(), Sprite { image: img.clone(), color, alpha_mode, ..default() }))
           .id()
    };
    let red = spawn(&mut app, Color::srgb(1., 0., 0.), SpriteAlphaMode::Blend);
    let red2 = spawn(&mut app, Color::srgb(1., 0., 0.), SpriteAlphaMode::Blend);
    let masked = spawn(&mut app, Color::WHITE, SpriteAlphaMode::Mask(0.5));
    app.update();

    assert_eq!(material(&app, red).base_color.to_linear(), LinearRgba::RED);
    assert_eq!(material(&app, red).alpha_mode, AlphaMode::Blend);
    assert_eq!(material(&app, masked).alpha_mode, AlphaMode::Mask(0.5));

    // materials are shared through the cache
    let handle = |app: &App, e| app.world().get::<MeshMaterial3d<StandardMaterial>>(e).unwrap().0.clone();
    assert_eq!(handle(&app, red), handle(&app, red2));
    assert_ne!(handle(&app, red), handle(&app, masked));

    // changing Sprite3d re-syncs the material too
    app.world_mut().get_mut::<Sprite3d>(red).unwrap().unlit = true;
    app.update();
    assert!(material(&app, red).unlit);
    assert_ne!(handle(&app, red), handle(&app, red2));

    // materials nobody uses any more are dropped
    let count = |app: &App| app.world().resource::<Assets<StandardMaterial>>().len();
    assert_eq!(count(&app), 3);
    app.world_mut().get_mut::<Sprite3d>(red2).unwrap().unlit = true;
    app.update();
    app.update(); // (the asset is removed a frame after its last handle drops)
    assert_eq!(count(&app), 2);
    assert_eq!(handle(&app, red), handle(&app, red2));

    // ...and its cache entry is swept the frame after, on the Unused event
    let cached = |app: &App| app.world().resource::<Sprite3dCaches>().material_cache.len();
    assert_eq!(cached(&app), 3);
    app.update();
    assert_eq!(cached(&app), 2);
}
