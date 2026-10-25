use bevy::asset::{AssetEventSystems, LoadState, RenderAssetUsages, StrongHandle, UntypedAssetId};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, VisibilitySystems};
use bevy::mesh::*;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::sprite::{Anchor, SpriteAlphaMode};
use std::hash::Hash;
use std::sync::{Arc, Weak};

pub mod prelude;

pub struct Sprite3dPlugin;
#[rustfmt::skip]
impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprite3dCaches>();
        app.register_type::<Sprite3d>();
        app.configure_sets(PostUpdate, Sprite3dSystems.before(VisibilitySystems::CalculateBounds));

        // Meshes and materials must be added before AssetEventSystems flushes
        // their Added events. bevy_pbr drops (and never retries) the pipeline-
        // specialization of an entity whose new mesh isn't in the render-world
        // yet, so an Image / TextureAtlasLayout modified in-place (hot reload,
        // etc.) resizes its sprites a frame late.
        // (Note: Changes to Sprite itself (atlas index, etc.) still land the same frame).
        // If bevy_pbr changes its code, this can go back after AssetEventSystems.
        app.add_systems(PostUpdate, (resync_modified_assets, sync_sprites)
                                        .chain()
                                        .before(AssetEventSystems)
                                        .in_set(Sprite3dSystems));
        app.add_observer(strip_mesh2d);
    }
}

/// Order against this set to see the current frame's meshes.
/// (eg. examples/dungeon.rs copies them onto outline entities)
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sprite3dSystems;

// Bevy's sprite renderer adds a Mesh2d to every Sprite. On a Sprite3d a
// Camera2d would draw it, and mesh picking treats anything with a Mesh2d as
// two-sided, so back-facing single-sided quads would still catch pointer rays.
//
// (Note: must go in the same command flush that added it)
fn strip_mesh2d(add: On<Add<(Mesh2d, Sprite3d)>>,
                query: Query<(), (With<Sprite3d>, With<Mesh2d>)>,
                mut commands: Commands)
{
    if query.contains(add.entity) {
        commands.entity(add.entity).remove::<Mesh2d>();
    }
}


// sizes (in texels) and anchors are multiplied by this, then cast to ints to
// query the mesh hashmap.
const MESH_CACHE_GRANULARITY: f32 = 1000.;
// colours likewise
const COLOUR_GRANULARITY: f32 = 1024.;

#[derive(Eq, Hash, PartialEq)]
pub struct MatKey
{
    image:      AssetId<Image>,
    alpha_mode: HashableAlphaMode,
    unlit:      bool,
    emissive:   [i32; 4],
    color:      [i32; 4],
}

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
fn reduce_colour(c: LinearRgba) -> [i32; 4] { [
        (c.red * COLOUR_GRANULARITY).round() as i32,
        (c.green * COLOUR_GRANULARITY).round() as i32,
        (c.blue * COLOUR_GRANULARITY).round() as i32,
        (c.alpha * COLOUR_GRANULARITY).round() as i32,
] }

fn expand_colour(c: [i32; 4]) -> LinearRgba
{
    LinearRgba::from_f32_array(c.map(|x| x as f32 / COLOUR_GRANULARITY))
}


// Materials are weak handles, so an image is dropped once no sprite uses it.
// A dead entry is overwritten the next time its key is looked up.
//
// (The maps are SwissTables (single allocation holding a control byte per
//  bucket and a flat array of (key, handle) buckets). Capacity only grows,
//  so no steady-state allocation churn.)
#[derive(Resource, Default)]
pub struct Sprite3dCaches
{
    pub mesh_cache:     HashMap<[u32; 12], Handle<Mesh>>,
    pub material_cache: HashMap<MatKey, Weak<StrongHandle>>,
}

impl Sprite3dCaches
{
    /// Drops every entry no sprite currently holds. (e.g. at a level transition)
    pub fn purge(&mut self)
    {
        self.mesh_cache.retain(|_, h| {
            let Handle::Strong(arc) = h else { unreachable!() };
            Arc::strong_count(arc) > 1 // held by something besides the cache
        });
        self.material_cache.retain(|_, w| w.strong_count() > 0);
    }
}

#[rustfmt::skip]
fn cached<K: Eq + Hash, A: Asset>(cache: &mut HashMap<K, Weak<StrongHandle>>,
                                  key: K,
                                  assets: &mut Assets<A>,
                                  build: impl FnOnce() -> A)
                                  -> Handle<A>
{
    if let Some(strong) = cache.get(&key).and_then(Weak::upgrade) {
        return Handle::Strong(strong);
    }
    let handle = assets.add(build());
    let Handle::Strong(strong) = &handle else { unreachable!() };
    cache.insert(key, Arc::downgrade(strong));
    handle
}

// The mesh and bounds for a sprite's current image / atlas frame, shared
// through the cache with every other sprite of the same shape. None while the
// image or atlas layout is still loading.
#[rustfmt::skip]
fn sprite_mesh(sprite3d: &Sprite3d,
               sprite: &Sprite,
               anchor: Vec2,
               images: &Assets<Image>,
               atlas_layouts: &Assets<TextureAtlasLayout>,
               caches: &mut Sprite3dCaches,
               meshes: &mut Assets<Mesh>)
               -> Option<(Mesh3d, Aabb)>
{
    let size = images.get(&sprite.image)?.size_f32();

    // as with 2d sprites, `rect` is relative to the atlas frame
    let frame = match &sprite.texture_atlas {
        Some(atlas) => atlas_layouts.get(&atlas.layout)?.textures[atlas.index].as_rect(),
        None => Rect::from_corners(Vec2::ZERO, size),
    };
    let rect = match sprite.rect {
        Some(r) => Rect { min: frame.min + r.min, max: frame.min + r.max },
        None => frame,
    };
    let size_px = sprite.custom_size.unwrap_or(rect.size());

    // w & h are the world-space size of the sprite.
    let w = size_px.x / sprite3d.pixels_per_metre;
    let h = size_px.y / sprite3d.pixels_per_metre;
    let mut uv = Rect { min: rect.min / size,
                        max: rect.max / size };

    if sprite.flip_x { std::mem::swap(&mut uv.min.x, &mut uv.max.x); }
    if sprite.flip_y { std::mem::swap(&mut uv.min.y, &mut uv.max.y); }

    let q = |x: f32| (x * MESH_CACHE_GRANULARITY) as i32 as u32;
    let key = [q(size_px.x), q(size_px.y), sprite3d.pixels_per_metre.to_bits(),
               q(anchor.x), q(anchor.y),
               sprite3d.double_sided as u32 | (sprite.flip_x as u32) << 1 | (sprite.flip_y as u32) << 2,
               q(rect.min.x), q(rect.min.y), q(rect.max.x), q(rect.max.y),
               size.x as u32, size.y as u32];

    // (greatly reduces number of unique meshes for tilemaps, etc.)
    let mesh = Mesh3d(caches.mesh_cache.entry(key)
                            .or_insert_with(|| meshes.add(quad(w, h, anchor, uv, sprite3d.double_sided)))
                            .clone());

    let aabb = Aabb { center:       Vec3A::new(-anchor.x * w, -anchor.y * h, 0.),
                      half_extents: Vec3A::new(w / 2., h / 2., 0.) };
    Some((mesh, aabb))
}

#[rustfmt::skip]
fn sprite_material(sprite3d: &Sprite3d,
                   sprite: &Sprite,
                   caches: &mut Sprite3dCaches,
                   materials: &mut Assets<StandardMaterial>)
                   -> MeshMaterial3d<StandardMaterial>
{
    let alpha_mode = match sprite.alpha_mode {
        SpriteAlphaMode::Opaque => AlphaMode::Opaque,
        SpriteAlphaMode::Mask(cutoff) => AlphaMode::Mask(cutoff),
        SpriteAlphaMode::Blend => AlphaMode::Blend,
    };
    let key = MatKey { image:      sprite.image.id(),
                       alpha_mode: HashableAlphaMode(alpha_mode),
                       unlit:      sprite3d.unlit,
                       emissive:   reduce_colour(sprite3d.emissive),
                       color:      reduce_colour(sprite.color.to_linear()), };

    let (color, emissive) = (expand_colour(key.color), expand_colour(key.emissive));

    MeshMaterial3d(cached(&mut caches.material_cache, key, materials,
                          || build_material(sprite.image.clone(),
                                            alpha_mode,
                                            sprite3d.unlit,
                                            emissive,
                                            color)))
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
                                  &Anchor,
                                  &mut Mesh3d,
                                  &mut MeshMaterial3d<StandardMaterial>,
                                  &mut Aabb,
                                  Has<Sprite3dPending>),
                                 Or<(With<Sprite3dPending>,
                                     Changed<Sprite>,
                                     Changed<Sprite3d>,
                                     Changed<Anchor>)>>)
{
    for (e, sprite3d, sprite, anchor, mut mesh, mut mat, mut aabb, pending) in &mut query {
        match sprite_mesh(sprite3d, sprite, anchor.as_vec(),
                          &images, &atlas_layouts, &mut caches, &mut meshes) {
            None => {
                assert_loadable(&asset_server, sprite.image.id(), e);
                if let Some(atlas) = &sprite.texture_atlas {
                    assert_loadable(&asset_server, atlas.layout.id(), e);
                }
                if !pending { commands.entity(e).insert(Sprite3dPending); }
            }
            Some((m, a)) => {
                mesh.set_if_neq(m);
                aabb.set_if_neq(a);
                mat.set_if_neq(sprite_material(sprite3d, sprite, &mut caches, &mut materials));
                if pending { commands.entity(e).remove::<Sprite3dPending>(); }
            }
        }
    }
}




// creates a quad mesh facing +z, textured with the `uv` region of the image.
// anchor has an expected range of (-0.5,-0.5) to (0.5,0.5), (0,0) being
// centered (though you can go out of bounds without issue)
fn quad(w: f32, h: f32, anchor: Vec2, uv: Rect, double_sided: bool) -> Mesh
{
    // Set RenderAssetUsages to the default value. Maybe allow customization or
    // choose a better default?
    #[rustfmt::skip]
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let px = (anchor.x + 0.5) * w;
    let py = (anchor.y + 0.5) * h;
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

    // the rects are per-quad constants, for custom materials. (`uv` may be
    // flipped, so it's normalised first)
    let uv_rect = Rect::from_corners(uv.min, uv.max);
    #[rustfmt::skip]
    mesh.insert_attribute(SPRITE3D_ATTRIBUTE_RECT,
        vec![[-px, -py, w - px, h - py]; 8]
    );
    #[rustfmt::skip]
    mesh.insert_attribute(SPRITE3D_ATTRIBUTE_UV_RECT,
        vec![[uv_rect.min.x, uv_rect.min.y, uv_rect.max.x, uv_rect.max.y]; 8]
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
                  color: LinearRgba)
                  -> StandardMaterial
{
    StandardMaterial { base_color_texture: Some(image),
                       cull_mode: Some(Face::Back),
                       alpha_mode,
                       unlit,
                       perceptual_roughness: 0.5,
                       reflectance: 0.15,
                       emissive,
                       base_color: color.into(),
                       ..Default::default() }
}


// marks a sprite whose image or atlas layout hasn't finished loading, so it
// can't be given a mesh yet.
#[derive(Component, Default)]
struct Sprite3dPending;

/// Represents a 3D sprite. Changes to this component, or to the entity's
/// `Sprite`, are reflected in the rendered mesh and material each frame.
///
/// The sprite's image, texture_atlas, color, flip_x, flip_y, custom_size,
/// rect and alpha_mode are honoured, as is the entity's [Anchor].
///
/// Note: `Sprite::alpha_mode` defaults to `Blend`, which is more expensive and
/// sorts in the transparent pass. Most sprites are better off with
/// `SpriteAlphaMode::Mask(0.5)` or `Opaque`.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default, Clone, Debug)]
#[require(Sprite, Mesh3d, MeshMaterial3d<StandardMaterial>, Aabb, NoAutoAabb, Sprite3dPending)]
pub struct Sprite3d
{
    /// Whether the sprite should be rendered as unlit.
    /// `false` (default) allows for lighting.
    pub unlit: bool,

    /// An emissive colour, if the sprite should emit light.
    /// `LinearRgba::BLACK` (default) does nothing.
    pub emissive: LinearRgba,

    /// the number of pixels per metre of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_metre: f32,

    /// Whether the sprite should be rendered as double-sided.
    /// `true` (default) adds a second set of indices, describing the same tris
    /// in reverse order.
    pub double_sided: bool,
}

/// Vertex attributes on every sprite mesh, for custom materials (which
/// bind them with `.at_shader_location(..)` in Material::specialize, see
/// examples/dungeon.rs).
/// SPRITE3D_ATTRIBUTE_RECT is the quad in local space, (min.x, min.y, max.x, max.y).
pub const SPRITE3D_ATTRIBUTE_RECT: MeshVertexAttribute = rect_attribute("Sprite3d_Rect");
/// SPRITE3D_ATTRIBUTE_UV_RECT the region of the image drawn, unaffected by flip_x / flip_y (min.x, min.y, max.x, max.y).
pub const SPRITE3D_ATTRIBUTE_UV_RECT: MeshVertexAttribute = rect_attribute("Sprite3d_UvRect");

// FNV-1a of the name for material keys
const fn rect_attribute(name: &'static str) -> MeshVertexAttribute
{
    let b = name.as_bytes();
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0;
    while i < b.len() {
        h = (h ^ b[i] as u64).wrapping_mul(0x100_0000_01b3);
        i += 1;
    }
    MeshVertexAttribute::new(name, h, VertexFormat::Float32x4)
}

impl Default for Sprite3d
{
    fn default() -> Self
    {
        Self { pixels_per_metre:   100.,
               unlit:              false,
               double_sided:       true,
               emissive:           LinearRgba::BLACK, }
    }
}
