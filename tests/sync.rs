use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::MeshAabb;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_sprite3d::prelude::*;

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
    (aabb.half_extents * 2.0).truncate().into()
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
