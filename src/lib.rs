use bevy::prelude::*;
use bevy::render::{ mesh::*, render_resource::*, render_asset::RenderAssetUsages};
use bevy::platform_support::collections::hash_map::HashMap;
use std::hash::Hash;

pub mod prelude;

pub struct Sprite3dPlugin;
impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprite3dCaches>();
        app.add_systems(PostUpdate, handle_texture_atlases);
    }
}


// sizes are multiplied by this, then cast to ints to query the mesh hashmap.
const MESH_CACHE_GRANULARITY: f32 = 1000.;

use std::marker::PhantomData;
use bevy::ecs::system::SystemParam;

// everything needed to register a sprite, passed in one go.
#[derive(SystemParam)]
pub struct Sprite3dParams<'w, 's> {
    pub meshes        : ResMut<'w, Assets<Mesh>>,
    pub materials     : ResMut<'w, Assets<StandardMaterial>>,
    pub images        : ResMut<'w, Assets<Image>>,
    pub atlas_layouts : ResMut<'w, Assets<TextureAtlasLayout>>,
    pub caches        : ResMut<'w, Sprite3dCaches>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}

#[derive(Eq, Hash, PartialEq)]
pub struct MatKey {
    image: Handle<Image>,
    alpha_mode: HashableAlphaMode,
    unlit: bool,
    emissive: [u8; 4],
}

const DEFAULT_ALPHA_MODE: AlphaMode = AlphaMode::Mask(0.5);

#[derive(Eq, PartialEq)]
struct HashableAlphaMode(AlphaMode);

impl Hash for HashableAlphaMode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.0 {
            AlphaMode::Opaque => 0.hash(state),
            AlphaMode::Mask(f) => {
                1.hash(state);
                f.to_bits().hash(state);
            },
            AlphaMode::Blend => 2.hash(state),
            AlphaMode::Premultiplied => 3.hash(state),
            AlphaMode::Add => 4.hash(state),
            AlphaMode::Multiply => 5.hash(state),
            AlphaMode::AlphaToCoverage => 6.hash(state),
        }
    }
}


fn reduce_colour(c: LinearRgba) -> [u8; 4] { [
        (c.red * 255.) as u8,
        (c.green * 255.) as u8,
        (c.blue * 255.) as u8,
        (c.alpha * 255.) as u8,
] }


#[derive(Resource)]
pub struct Sprite3dCaches {
    pub mesh_cache: HashMap<[u32; 9], Mesh3d>,
    pub material_cache: HashMap<MatKey, MeshMaterial3d<StandardMaterial>>,
}

impl Default for Sprite3dCaches {
    fn default() -> Self {
        Sprite3dCaches {
            mesh_cache: HashMap::with_hasher(Default::default()),
            material_cache: HashMap::with_hasher(Default::default()),
        }
    }
}







// Update the mesh of a Sprite3d with an atlas sprite when its index changes.
fn handle_texture_atlases(
    caches: Res<Sprite3dCaches>,
    mut query: Query<(&mut Mesh3d, &Sprite3d), Changed<Sprite3d>>,
) {
    for (mut mesh, sprite_3d) in query.iter_mut() {
        let Some(texture_atlas) = &sprite_3d.texture_atlas else {
            continue;
        };
        let Some(mesh_keys) = &sprite_3d.texture_atlas_keys else {
            continue;
        };

        *mesh = caches.mesh_cache.get(&mesh_keys[texture_atlas.index]).unwrap().clone();
    }
}




// creates a (potentially offset) quad mesh facing +z
// pivot = None will have a center pivot
// pivot = Some(p) will have an expected range of p \in (0,0) to (1,1)
// (though you can go out of bounds without issue)
fn quad(w: f32, h: f32, pivot: Option<Vec2>, double_sided: bool) -> Mesh {
    let w2 = w / 2.0;
    let h2 = h / 2.0;

    // Set RenderAssetUsages to the default value. Maybe allow customization or
    // choose a better default?
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let vertices = match pivot {
        None => { vec![[-w2, -h2, 0.0], [w2, -h2, 0.0], [-w2, h2, 0.0], [w2, h2, 0.0],
                       [-w2, -h2, 0.0], [w2, -h2, 0.0], [-w2, h2, 0.0], [w2, h2, 0.0]] },
        Some(pivot) => {
            let px = pivot.x * w;
            let py = pivot.y * h;
            vec![[-px, -py, 0.0], [w - px, -py, 0.0], [-px, h - py, 0.0], [w - px, h - py, 0.0],
                 [-px, -py, 0.0], [w - px, -py, 0.0], [-px, h - py, 0.0], [w - px, h - py, 0.0]]
        }
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0,  1.0], [0.0, 0.0,  1.0], [0.0, 0.0,  1.0], [0.0, 0.0,  1.0],
                                                       [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0]]);

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0],
                                                     [0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]]);

    mesh.insert_indices(Indices::U32(
        if double_sided { vec![0, 1, 2, 1, 3, 2, 5, 4, 6, 7, 5, 6] }
        else {            vec![0, 1, 2, 1, 3, 2] }
    ));

    mesh
}




// generate a StandardMaterial useful for rendering a sprite
fn material(image: Handle<Image>, alpha_mode: AlphaMode, unlit: bool, emissive: LinearRgba) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(image),
        cull_mode: Some(Face::Back),
        alpha_mode,
        unlit,
        perceptual_roughness: 0.5,
        reflectance: 0.15,
        emissive,

        ..Default::default()
    }
}




/// A precursor struct for a sprite. Set necessary parameters manually, use
/// `..default()` for others, then call `bundle()` to to get a bundle
/// that can be spawned into the world.
pub struct Sprite3dBuilder {
    /// the sprite image. See `readme.md` for examples.
    pub image: Handle<Image>,

    // TODO: ability to specify exact size, with None scaled by image's ratio and other.

    /// the number of pixels per metre of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_metre: f32,

    /// The sprite's pivot. eg. the point specified by the sprite's
    /// transform, around which a rotation will be performed.
    ///
    /// - pivot = None will have a center pivot
    /// - pivot = Some(p) will have an expected range of p \in `(0,0)` to `(1,1)`
    ///   (though you can go out of bounds without issue)
    pub pivot: Option<Vec2>,

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

    /// Whether the sprite should be rendered as double-sided.
    /// `true` (default) adds a second set of indices, describing the same tris
    /// in reverse order.
    pub double_sided: bool,

    /// An emissive colour, if the sprite should emit light.
    /// `LinearRgba::Black` (default) does nothing.
    pub emissive: LinearRgba,
}

impl Default for Sprite3dBuilder {
    fn default() -> Self {
        Self {
            image: Default::default(),
            pixels_per_metre: 100.,
            pivot: None,
            alpha_mode: DEFAULT_ALPHA_MODE,
            unlit: false,
            double_sided: true,
            emissive: LinearRgba::BLACK,
        }
    }
}


/// Represents a 3D sprite. May store texture atlas data -- note that modifying
/// `texture_atlas` and `texture_atlas_keys` on an already spawned sprite may
/// cause buggy behavior.
#[derive(Component)]
#[require(Transform, Mesh3d)]
pub struct Sprite3d {
    pub texture_atlas: Option<TextureAtlas>,
    pub texture_atlas_keys: Option<Vec<[u32; 9]>>,
}

#[derive(Bundle)]
pub struct Sprite3dBundle {
    pub sprite_3d: Sprite3d,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
}

impl Sprite3dBuilder {
    /// creates a bundle of components
    pub fn bundle(self, params: &mut Sprite3dParams) -> Sprite3dBundle {
        // get image dimensions
        let image_size = params.images.get(&self.image).unwrap().texture_descriptor.size;
        // w & h are the world-space size of the sprite.
        let w = (image_size.width  as f32) / self.pixels_per_metre;
        let h = (image_size.height as f32) / self.pixels_per_metre;

        Sprite3dBundle {
            sprite_3d: Sprite3d {
                texture_atlas: None,
                texture_atlas_keys: None,
            },
            mesh: {
                let pivot = self.pivot.unwrap_or(Vec2::new(0.5, 0.5));

                let mesh_key = [(w * MESH_CACHE_GRANULARITY) as u32,
                                (h * MESH_CACHE_GRANULARITY) as u32,
                                (pivot.x * MESH_CACHE_GRANULARITY) as u32,
                                (pivot.y * MESH_CACHE_GRANULARITY) as u32,
                                self.double_sided as u32,
                                0, 0, 0, 0
                                ];

                // if we have a mesh in the cache, use it.
                // (greatly reduces number of unique meshes for tilemaps, etc.)
                if let Some(mesh) = params.caches.mesh_cache.get(&mesh_key) { mesh.clone() }
                else { // otherwise, create a new mesh and cache it.
                    let mesh = Mesh3d(params.meshes.add(quad( w, h, self.pivot, self.double_sided )));
                    params.caches.mesh_cache.insert(mesh_key, mesh.clone());
                    mesh
                }
            },
            // likewise for material, use the existing if the image is already cached.
            // (possibly look into a bool in Sprite3dBuilder to manually disable caching for an individual sprite?)
            material: {
                let mat_key = MatKey {
                    image: self.image.clone(),
                    alpha_mode: HashableAlphaMode(self.alpha_mode),
                    unlit: self.unlit,
                    emissive: reduce_colour(self.emissive),
                };

                if let Some(material) = params.caches.material_cache.get(&mat_key) { material.clone() }
                else {
                    let material = MeshMaterial3d(params.materials.add(material(self.image.clone(), self.alpha_mode, self.unlit, self.emissive)));
                    params.caches.material_cache.insert(mat_key, material.clone());
                    material
                }
            },
        }
    }

    /// creates a bundle of components with support for texture atlases
    pub fn bundle_with_atlas(
        self,
        params: &mut Sprite3dParams,
        atlas: TextureAtlas,
    ) -> Sprite3dBundle {
        let atlas_layout = params.atlas_layouts.get(&atlas.layout).unwrap();
        let image = params.images.get(&self.image).unwrap();
        let image_size = image.texture_descriptor.size;

        let pivot = self.pivot.unwrap_or(Vec2::new(0.5, 0.5));
        // cache all the meshes for the atlas (if they haven't been already)
        // so that we can change the index later and not have to re-create the mesh.

        // store all lookup keys in a vec so we later know which meshes to retrieve.
        let mut mesh_keys = Vec::new();


        for i in 0..atlas_layout.textures.len() {

            let rect = atlas_layout.textures[i];

            let w = rect.width() as f32 / self.pixels_per_metre;
            let h = rect.height() as f32 / self.pixels_per_metre;

            let frac_rect = bevy::math::Rect {
                min: Vec2::new(rect.min.x as f32 / (image_size.width as f32),
                               rect.min.y as f32 / (image_size.height as f32)),

                max: Vec2::new(rect.max.x as f32 / (image_size.width as f32),
                               rect.max.y as f32 / (image_size.height as f32)),
            };

            let mut rect_pivot = pivot;

            // scale pivot to be relative to the rect within the atlas.
            rect_pivot.x *= frac_rect.width();
            rect_pivot.y *= frac_rect.height();
            rect_pivot += frac_rect.min;


            let mesh_key = [(w * MESH_CACHE_GRANULARITY) as u32,
                            (h * MESH_CACHE_GRANULARITY) as u32,
                            (rect_pivot.x * MESH_CACHE_GRANULARITY) as u32,
                            (rect_pivot.y * MESH_CACHE_GRANULARITY) as u32,
                            self.double_sided as u32,
                            (frac_rect.min.x * MESH_CACHE_GRANULARITY) as u32,
                            (frac_rect.min.y * MESH_CACHE_GRANULARITY) as u32,
                            (frac_rect.max.x * MESH_CACHE_GRANULARITY) as u32,
                            (frac_rect.max.y * MESH_CACHE_GRANULARITY) as u32];

            mesh_keys.push(mesh_key);

            // if we don't have a mesh in the cache, create it.
            if !params.caches.mesh_cache.contains_key(&mesh_key) {
                let mut mesh = quad( w, h, Some(pivot), self.double_sided );
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![
                    [frac_rect.min.x, frac_rect.max.y],
                    [frac_rect.max.x, frac_rect.max.y],
                    [frac_rect.min.x, frac_rect.min.y],
                    [frac_rect.max.x, frac_rect.min.y],

                    [frac_rect.min.x, frac_rect.max.y],
                    [frac_rect.max.x, frac_rect.max.y],
                    [frac_rect.min.x, frac_rect.min.y],
                    [frac_rect.max.x, frac_rect.min.y],
                ]);
                let mesh_h = Mesh3d(params.meshes.add(mesh));
                params.caches.mesh_cache.insert(mesh_key, mesh_h);
            }
        }

        Sprite3dBundle {
            mesh: params.caches.mesh_cache.get(&mesh_keys[atlas.index]).unwrap().clone(),
            material: {
                let mat_key = MatKey {
                    image: self.image.clone(),
                    alpha_mode: HashableAlphaMode(self.alpha_mode),
                    unlit: self.unlit,
                    emissive: reduce_colour(self.emissive),
                };
                if let Some(material) = params.caches.material_cache.get(&mat_key) { material.clone() }
                else {
                    let material = MeshMaterial3d(params.materials.add(material(self.image.clone(), self.alpha_mode, self.unlit, self.emissive)));
                    params.caches.material_cache.insert(mat_key, material.clone());
                    material
                }
            },
            sprite_3d: Sprite3d {
                texture_atlas: Some(atlas),
                texture_atlas_keys: Some(mesh_keys),
            },
        }
    }
}
