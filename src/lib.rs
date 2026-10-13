use bevy::asset::{AssetEventSystems, LoadState, RenderAssetUsages, UntypedAssetId};
use bevy::mesh::*;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::*;
use std::hash::Hash;

pub mod prelude;

pub struct Sprite3dPlugin;
#[rustfmt::skip]
impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprite3dCaches>();
        app.add_systems(PostUpdate, (resync_modified_assets, sync_sprites)
                                        .chain()
                                        .after(AssetEventSystems));
    }
}


// sizes are multiplied by this, then cast to ints to query the mesh hashmap.
const MESH_CACHE_GRANULARITY: f32 = 1000.;

#[derive(Eq, Hash, PartialEq)]
pub struct MatKey
{
    image:      Handle<Image>,
    alpha_mode: HashableAlphaMode,
    unlit:      bool,
    emissive:   [u8; 4],
    flip_x:     bool,
    flip_y:     bool,
}

const DEFAULT_ALPHA_MODE: AlphaMode = AlphaMode::Mask(0.5);

#[derive(Eq, PartialEq)]
struct HashableAlphaMode(AlphaMode);

impl Hash for HashableAlphaMode
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        match self.0 {
            AlphaMode::Opaque => 0.hash(state),
            AlphaMode::Mask(f) => {
                1.hash(state);
                f.to_bits().hash(state);
            }
            AlphaMode::Blend => 2.hash(state),
            AlphaMode::Premultiplied => 3.hash(state),
            AlphaMode::Add => 4.hash(state),
            AlphaMode::Multiply => 5.hash(state),
            AlphaMode::AlphaToCoverage => 6.hash(state),
        }
    }
}


#[rustfmt::skip]
fn reduce_colour(c: LinearRgba) -> [u8; 4] { [
        (c.red * 255.) as u8,
        (c.green * 255.) as u8,
        (c.blue * 255.) as u8,
        (c.alpha * 255.) as u8,
] }


#[derive(Resource, Default)]
pub struct Sprite3dCaches
{
    pub mesh_cache:     HashMap<[u32; 9], Mesh3d>,
    pub material_cache: HashMap<MatKey, MeshMaterial3d<StandardMaterial>>,
}

// The mesh for a sprite's current image / atlas frame, shared through the
// cache with every other sprite of the same shape. None while the image or
// atlas layout is still loading.
#[rustfmt::skip]
fn sprite_mesh(sprite3d: &Sprite3d,
               sprite: &Sprite,
               images: &Assets<Image>,
               atlas_layouts: &Assets<TextureAtlasLayout>,
               caches: &mut Sprite3dCaches,
               meshes: &mut Assets<Mesh>)
               -> Option<Mesh3d>
{
    let size = images.get(&sprite.image)?.texture_descriptor.size;
    let image_size = Vec2::new(size.width as f32, size.height as f32);

    let rect = match &sprite.texture_atlas {
        Some(atlas) => atlas_layouts.get(&atlas.layout)?.textures[atlas.index],
        None => URect::new(0, 0, size.width, size.height),
    };

    // w & h are the world-space size of the sprite.
    let w = rect.width() as f32 / sprite3d.pixels_per_metre;
    let h = rect.height() as f32 / sprite3d.pixels_per_metre;
    let pivot = sprite3d.pivot.unwrap_or(Vec2::splat(0.5));
    let uv = Rect { min: rect.min.as_vec2() / image_size,
                    max: rect.max.as_vec2() / image_size };

    let q = |x: f32| (x * MESH_CACHE_GRANULARITY) as i32 as u32;
    let key = [q(w), q(h), q(pivot.x), q(pivot.y), sprite3d.double_sided as u32,
               q(uv.min.x), q(uv.min.y), q(uv.max.x), q(uv.max.y)];

    // (greatly reduces number of unique meshes for tilemaps, etc.)
    Some(caches.mesh_cache.entry(key).or_insert_with(|| {
        Mesh3d(meshes.add(quad(w, h, pivot, uv, sprite3d.double_sided)))
    }).clone())
}

#[rustfmt::skip]
fn sprite_material(sprite3d: &Sprite3d,
                   sprite: &Sprite,
                   caches: &mut Sprite3dCaches,
                   materials: &mut Assets<StandardMaterial>)
                   -> MeshMaterial3d<StandardMaterial>
{
    let key = MatKey { image:      sprite.image.clone(),
                       alpha_mode: HashableAlphaMode(sprite3d.alpha_mode),
                       unlit:      sprite3d.unlit,
                       emissive:   reduce_colour(sprite3d.emissive),
                       flip_x:     sprite.flip_x,
                       flip_y:     sprite.flip_y, };
    caches.material_cache.entry(key).or_insert_with(|| {
        MeshMaterial3d(materials.add(build_material(sprite.image.clone(),
                                                    sprite3d.alpha_mode,
                                                    sprite3d.unlit,
                                                    sprite3d.emissive,
                                                    sprite.flip_x,
                                                    sprite.flip_y)))
    }).clone()
}

fn assert_loadable(asset_server: &AssetServer, id: impl Into<UntypedAssetId>, e: Entity)
{
    if let Some(LoadState::Failed(err)) = asset_server.get_load_state(id) {
        panic!("bevy_sprite3d: an asset used by sprite {e} failed to load: {err}");
    }
}

// Re-sync sprites whose image or atlas layout was modified.
#[rustfmt::skip]
fn resync_modified_assets(mut commands: Commands,
                          mut images: MessageReader<AssetEvent<Image>>,
                          mut layouts: MessageReader<AssetEvent<TextureAtlasLayout>>,
                          query: Query<(Entity, &Sprite), With<Sprite3d>>)
{
    fn modified<A: Asset>(events: &mut MessageReader<AssetEvent<A>>) -> HashSet<AssetId<A>>
    {
        events.read()
              .filter_map(|ev| match ev { AssetEvent::Modified { id } => Some(*id), _ => None })
              .collect()
    }
    let images = modified(&mut images);
    let layouts = modified(&mut layouts);
    if images.is_empty() && layouts.is_empty() { return; }

    for (e, sprite) in &query {
        if images.contains(&sprite.image.id())
        || sprite.texture_atlas.as_ref().is_some_and(|a| layouts.contains(&a.layout.id())) {
            commands.entity(e).insert(Sprite3dPending);
        }
    }
}

// Assign mesh and material to every sprite that is new, changed, or still
// waiting on its assets.
#[rustfmt::skip]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_sprites(mut commands: Commands,
                asset_server: Res<AssetServer>,
                images: Res<Assets<Image>>,
                atlas_layouts: Res<Assets<TextureAtlasLayout>>,
                mut caches: ResMut<Sprite3dCaches>,
                mut meshes: ResMut<Assets<Mesh>>,
                mut materials: ResMut<Assets<StandardMaterial>>,
                mut query: Query<(Entity,
                                  &Sprite3d,
                                  &Sprite,
                                  &mut Mesh3d,
                                  &mut MeshMaterial3d<StandardMaterial>,
                                  Has<Sprite3dPending>),
                                 Or<(With<Sprite3dPending>, Changed<Sprite>, Changed<Sprite3d>)>>)
{
    for (e, sprite3d, sprite, mut mesh, mut mat, pending) in &mut query {
        match sprite_mesh(sprite3d, sprite, &images, &atlas_layouts, &mut caches, &mut meshes) {
            None => {
                assert_loadable(&asset_server, sprite.image.id(), e);
                if let Some(atlas) = &sprite.texture_atlas {
                    assert_loadable(&asset_server, atlas.layout.id(), e);
                }
                if !pending { commands.entity(e).insert(Sprite3dPending); }
            }
            Some(m) => {
                mesh.set_if_neq(m);
                mat.set_if_neq(sprite_material(sprite3d, sprite, &mut caches, &mut materials));
                if pending { commands.entity(e).remove::<Sprite3dPending>(); }
            }
        }
    }
}




// creates a quad mesh facing +z, textured with the `uv` region of the image.
// pivot has an expected range of (0,0) to (1,1), (0.5, 0.5) being centered
// (though you can go out of bounds without issue)
fn quad(w: f32, h: f32, pivot: Vec2, uv: Rect, double_sided: bool) -> Mesh
{
    // Set RenderAssetUsages to the default value. Maybe allow customization or
    // choose a better default?
    #[rustfmt::skip]
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let px = pivot.x * w;
    let py = pivot.y * h;
    #[rustfmt::skip]
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,
        vec![[-px, -py, 0.0], [w - px, -py, 0.0], [-px, h - py, 0.0], [w - px, h - py, 0.0],
             [-px, -py, 0.0], [w - px, -py, 0.0], [-px, h - py, 0.0], [w - px, h - py, 0.0]]
    );

    #[rustfmt::skip]
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0,  1.0], [0.0, 0.0,  1.0], [0.0, 0.0,  1.0], [0.0, 0.0,  1.0],
             [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0]]
    );

    #[rustfmt::skip]
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,
        vec![[uv.min.x, uv.max.y], [uv.max.x, uv.max.y], [uv.min.x, uv.min.y], [uv.max.x, uv.min.y],
             [uv.min.x, uv.max.y], [uv.max.x, uv.max.y], [uv.min.x, uv.min.y], [uv.max.x, uv.min.y]]
    );

    #[rustfmt::skip]
    mesh.insert_indices(Indices::U32(
        if double_sided { vec![0, 1, 2, 1, 3, 2, 5, 4, 6, 7, 5, 6] }
        else {            vec![0, 1, 2, 1, 3, 2] }
    ));

    mesh
}




// generate a StandardMaterial useful for rendering a sprite
fn build_material(image: Handle<Image>,
                  alpha_mode: AlphaMode,
                  unlit: bool,
                  emissive: LinearRgba,
                  flip_x: bool,
                  flip_y: bool)
                  -> StandardMaterial
{
    let mut mat = StandardMaterial { base_color_texture: Some(image),
                                     cull_mode: Some(Face::Back),
                                     alpha_mode,
                                     unlit,
                                     perceptual_roughness: 0.5,
                                     reflectance: 0.15,
                                     emissive,
                                     ..Default::default() };
    mat.flip(flip_x, flip_y);
    mat
}


// marks a sprite whose image or atlas layout hasn't finished loading, so it
// can't be given a mesh yet.
#[derive(Component, Default)]
struct Sprite3dPending;

/// Represents a 3D sprite. Changes to this component, or to the entity's
/// `Sprite`, are reflected in the rendered mesh and material each frame.
#[derive(Component)]
#[require(Transform, Mesh3d, MeshMaterial3d<StandardMaterial>, Sprite3dPending)]
pub struct Sprite3d
{
    /// The sprite's alpha mode.
    ///
    /// - `Mask(0.5)` (default) only allows fully opaque or fully transparent pixels
    ///   (cutoff at `0.5`).
    /// - `Blend` allows partially transparent pixels (slightly more expensive).
    /// - Use any other value to achieve desired blending effect.
    pub alpha_mode: AlphaMode,

    /// Whether the sprite should be rendered as unlit.
    /// `false` (default) allows for lighting.
    pub unlit: bool,

    /// An emissive colour, if the sprite should emit light.
    /// `LinearRgba::Black` (default) does nothing.
    pub emissive: LinearRgba,

    /// the number of pixels per metre of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_metre: f32,

    /// The sprite's pivot. eg. the point specified by the sprite's
    /// transform, around which a rotation will be performed.
    ///
    /// - pivot = None will have a center pivot
    /// - pivot = Some(p) will have an expected range of p \in `(0,0)` to `(1,1)`
    ///   (though you can go out of bounds without issue)
    pub pivot: Option<Vec2>,

    /// Whether the sprite should be rendered as double-sided.
    /// `true` (default) adds a second set of indices, describing the same tris
    /// in reverse order.
    pub double_sided: bool,
}

impl Default for Sprite3d
{
    fn default() -> Self
    {
        Self { pixels_per_metre:   100.,
               pivot:              None,
               alpha_mode:         DEFAULT_ALPHA_MODE,
               unlit:              false,
               double_sided:       true,
               emissive:           LinearRgba::BLACK, }
    }
}
