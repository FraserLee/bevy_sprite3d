use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::{mesh::*, render_resource::*};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;

pub struct Sprite3dPlugin;
impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprite3dRes>();
        app.add_systems(PostUpdate, sprite3d_system);
    }
}

// sizes are multiplied by this, then cast to ints to query the mesh hashmap.
const MESH_CACHE_GRANULARITY: f32 = 1000.;

// everything needed to register a sprite, passed in one go.
#[derive(SystemParam)]
pub struct Sprite3dParams<'w, 's> {
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub atlases: ResMut<'w, Assets<TextureAtlas>>,
    pub sr: ResMut<'w, Sprite3dRes>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}

#[derive(Eq, Hash, PartialEq)]
pub struct MatKey {
    image: Handle<Image>,
    partial_alpha: bool,
    unlit: bool,
    emissive: [u8; 4],
}

fn reduce_colour(c: Color) -> [u8; 4] {
    [
        (c.r() * 255.) as u8,
        (c.g() * 255.) as u8,
        (c.b() * 255.) as u8,
        (c.a() * 255.) as u8,
    ]
}

#[derive(Default, Resource)]
pub struct Sprite3dRes {
    pub mesh_cache: HashMap<[u32; 9], Handle<Mesh>>,
    pub material_cache: HashMap<MatKey, Handle<StandardMaterial>>,
}

// Update the mesh of any AtlasSprite3d when its index changes.
fn sprite3d_system(
    sprite_params: Sprite3dParams,
    mut query: Query<(&mut Handle<Mesh>, &AtlasSprite3dComponent), Changed<AtlasSprite3dComponent>>,
) {
    for (mut mesh, params) in query.iter_mut() {
        // this unwrap will always succeed, unless mesh_cache is corrupted.
        *mesh = sprite_params
            .sr
            .mesh_cache
            .get(&params.atlas[params.index])
            .unwrap()
            .clone();
    }
}

// creates a (potentially offset) quad mesh facing +z
// pivot = None will have a center pivot
// pivot = Some(p) will have an expected range of p \in (0,0) to (1,1)
// (though you can go out of bounds without issue)
fn quad(w: f32, h: f32, pivot: Option<Vec2>, double_sided: bool) -> Mesh {
    let w2 = w / 2.0;
    let h2 = h / 2.0;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let vertices = match pivot {
        None => {
            vec![
                [-w2, -h2, 0.0],
                [w2, -h2, 0.0],
                [-w2, h2, 0.0],
                [w2, h2, 0.0],
                [-w2, -h2, 0.0],
                [w2, -h2, 0.0],
                [-w2, h2, 0.0],
                [w2, h2, 0.0],
            ]
        }
        Some(pivot) => {
            let px = pivot.x * w;
            let py = pivot.y * h;
            vec![
                [-px, -py, 0.0],
                [w - px, -py, 0.0],
                [-px, h - py, 0.0],
                [w - px, h - py, 0.0],
                [-px, -py, 0.0],
                [w - px, -py, 0.0],
                [-px, h - py, 0.0],
                [w - px, h - py, 0.0],
            ]
        }
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
        ],
    );

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.0, 1.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0],
        ],
    );

    mesh.set_indices(Some(Indices::U32(if double_sided {
        vec![0, 1, 2, 1, 3, 2, 5, 4, 6, 7, 5, 6]
    } else {
        vec![0, 1, 2, 1, 3, 2]
    })));

    mesh
}

// generate a StandardMaterial useful for rendering a sprite
fn material(
    image: Handle<Image>,
    partial_alpha: bool,
    unlit: bool,
    emissive: Color,
) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(image),
        cull_mode: Some(Face::Back),
        alpha_mode: if partial_alpha {
            AlphaMode::Blend
        } else {
            AlphaMode::Mask(0.5)
        },
        unlit,
        perceptual_roughness: 0.5,
        reflectance: 0.15,
        emissive,

        ..Default::default()
    }
}

/// A precursor struct for a sprite. Set necessary parameters manually, use
/// `..default()` for others, then call `bundle()` to to get a `PBRBundle`
/// that can be spawned into the world.
pub struct Sprite3d {
    /// the sprite's transform
    pub transform: Transform,

    /// the sprite image. See `readme.md` for examples.
    pub image: Handle<Image>,

    // TODO: ability to specify exact size, with None scaled by image's ratio and other.
    /// the number of pixels per meter of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_meter: f32,

    /// The sprite's pivot. eg. the point specified by the sprite's
    /// transform, around which a rotation will be performed.
    ///
    /// - pivot = None will have a center pivot
    /// - pivot = Some(p) will have an expected range of p \in `(0,0)` to `(1,1)`
    ///   (though you can go out of bounds without issue)
    pub pivot: Option<Vec2>,

    /// Whether the sprite should support partial alpha.
    ///
    /// - `false` (default) only allows fully opaque or fully transparent pixels
    ///   (cutoff at `0.5`).
    /// - `true` allows partially transparent pixels
    ///   (slightly more expensive, so disabled when not needed).
    pub partial_alpha: bool,

    /// Whether the sprite should be rendered as unlit.
    /// `false` (default) allows for lighting.
    pub unlit: bool,

    /// Whether the sprite should be rendered as double-sided.
    /// `true` (default) adds a second set of indices, describing the same tris
    /// in reverse order.
    pub double_sided: bool,

    /// An emissive colour, if the sprite should emit light.
    /// `Color::Black` (default) does nothing.
    pub emissive: Color,
}

impl Default for Sprite3d {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            image: Default::default(),
            pixels_per_meter: 100.,
            pivot: None,
            partial_alpha: false,
            unlit: false,
            double_sided: true,
            emissive: Color::BLACK,
        }
    }
}

// just a marker for queries at the moment, could be expanded later if needed.
#[derive(Component)]
pub struct Sprite3dComponent {}

#[derive(Bundle)]
pub struct Sprite3dBundle {
    pub params: Sprite3dComponent,
    pub pbr: PbrBundle,
}

impl Sprite3d {
    /// creates a bundle of components from the Sprite3d struct.
    pub fn bundle(self, params: &mut Sprite3dParams) -> Sprite3dBundle {
        // get image dimensions
        let image_size = params
            .images
            .get(&self.image)
            .unwrap()
            .texture_descriptor
            .size;
        // w & h are the world-space size of the sprite.
        let w = (image_size.width as f32) / self.pixels_per_meter;
        let h = (image_size.height as f32) / self.pixels_per_meter;

        return Sprite3dBundle {
            params: Sprite3dComponent {},
            pbr: PbrBundle {
                mesh: {
                    let pivot = self.pivot.unwrap_or(Vec2::new(0.5, 0.5));

                    let mesh_key = [
                        (w * MESH_CACHE_GRANULARITY) as u32,
                        (h * MESH_CACHE_GRANULARITY) as u32,
                        (pivot.x * MESH_CACHE_GRANULARITY) as u32,
                        (pivot.y * MESH_CACHE_GRANULARITY) as u32,
                        self.double_sided as u32,
                        0,
                        0,
                        0,
                        0,
                    ];

                    // if we have a mesh in the cache, use it.
                    // (greatly reduces number of unique meshes for tilemaps, etc.)
                    if let Some(mesh) = params.sr.mesh_cache.get(&mesh_key) {
                        mesh.clone()
                    } else {
                        // otherwise, create a new mesh and cache it.
                        let mesh = params.meshes.add(quad(w, h, self.pivot, self.double_sided));
                        params.sr.mesh_cache.insert(mesh_key, mesh.clone());
                        mesh
                    }
                },

                // likewise for material, use the existing if the image is already cached.
                // (possibly look into a bool in Sprite3d to manually disable caching for an individual sprite?)
                material: {
                    let mat_key = MatKey {
                        image: self.image.clone(),
                        partial_alpha: self.partial_alpha,
                        unlit: self.unlit,
                        emissive: reduce_colour(self.emissive),
                    };

                    if let Some(material) = params.sr.material_cache.get(&mat_key) {
                        material.clone()
                    } else {
                        let material = params.materials.add(material(
                            self.image.clone(),
                            self.partial_alpha,
                            self.unlit,
                            self.emissive,
                        ));
                        params.sr.material_cache.insert(mat_key, material.clone());
                        material
                    }
                },

                transform: self.transform,

                ..default()
            },
        };
    }
}

/// Same as Sprite3d, but for sprites in a texture atlas.
///
/// A precursor struct for a sprite. Set necessary parameters manually, use
/// `..default()` for others, then call `bundle()` to to get a `PBRBundle`
/// that can be spawned into the world.
pub struct AtlasSprite3d {
    /// the sprite's transform
    pub transform: Transform,
    /// the sprite texture atlas. See `readme.md` for examples.
    pub atlas: Handle<TextureAtlas>,
    /// the sprite's index in the atlas.
    pub index: usize,

    /// the number of pixels per meter of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_meter: f32,

    /// The sprite's pivot. eg. the point specified by the sprite's
    /// transform, around which a rotation will be performed.
    ///
    /// - pivot = None will have a center pivot
    /// - pivot = Some(p) will have an expected range of p \in `(0,0)` to `(1,1)`
    ///   (though you can go out of bounds without issue)
    pub pivot: Option<Vec2>,

    /// Whether the sprite should support partial alpha.
    ///
    /// - `false` (default) only allows fully opaque or fully transparent pixels
    ///   (cutoff at `0.5`).
    /// - `true` allows partially transparent pixels
    ///   (slightly more expensive, so disabled when not needed).
    pub partial_alpha: bool,

    /// Whether the sprite should be rendered as unlit.
    /// `false` (default) allows for lighting.
    pub unlit: bool,

    /// Whether the sprite should be rendered as double-sided.
    /// `true` (default) adds a second set of indices, describing the same tris
    /// in reverse order.
    pub double_sided: bool,

    /// An emissive colour, if the sprite should emit light.
    /// `Color::Black` (default) does nothing.
    pub emissive: Color,
}

impl Default for AtlasSprite3d {
    fn default() -> AtlasSprite3d {
        AtlasSprite3d {
            transform: Transform::default(),
            atlas: Handle::<TextureAtlas>::default(),
            index: 0,
            pixels_per_meter: 100.,
            pivot: None,
            partial_alpha: false,
            unlit: false,
            double_sided: true,
            emissive: Color::BLACK,
        }
    }
}

#[derive(Component)]
pub struct AtlasSprite3dComponent {
    pub index: usize,
    pub atlas: Vec<[u32; 9]>,
}

#[derive(Bundle)]
pub struct AtlasSprite3dBundle {
    pub params: AtlasSprite3dComponent,
    pub pbr: PbrBundle,
}

impl AtlasSprite3d {
    /// creates a bundle of components from the AtlasSprite3d struct.
    pub fn bundle(self, params: &mut Sprite3dParams) -> AtlasSprite3dBundle {
        let atlas = params.atlases.get(&self.atlas).unwrap();
        let image = params.images.get(&atlas.texture).unwrap();
        let image_size = image.texture_descriptor.size;

        let pivot = self.pivot.unwrap_or(Vec2::new(0.5, 0.5));
        // cache all the meshes for the atlas (if they haven't been already)
        // so that we can change the index later and not have to re-create the mesh.

        // store all lookup keys in a vec so we later know which meshes to retrieve.
        let mut mesh_keys = Vec::new();

        for i in 0..atlas.textures.len() {
            let rect = atlas.textures[i];

            let w = rect.width() / self.pixels_per_meter;
            let h = rect.height() / self.pixels_per_meter;

            let frac_rect = bevy::math::Rect {
                min: Vec2::new(
                    rect.min.x / (image_size.width as f32),
                    rect.min.y / (image_size.height as f32),
                ),

                max: Vec2::new(
                    rect.max.x / (image_size.width as f32),
                    rect.max.y / (image_size.height as f32),
                ),
            };

            let mut rect_pivot = pivot;

            // scale pivot to be relative to the rect within the atlas.
            rect_pivot.x *= frac_rect.width();
            rect_pivot.y *= frac_rect.height();
            rect_pivot += frac_rect.min;

            let mesh_key = [
                (w * MESH_CACHE_GRANULARITY) as u32,
                (h * MESH_CACHE_GRANULARITY) as u32,
                (rect_pivot.x * MESH_CACHE_GRANULARITY) as u32,
                (rect_pivot.y * MESH_CACHE_GRANULARITY) as u32,
                self.double_sided as u32,
                (frac_rect.min.x * MESH_CACHE_GRANULARITY) as u32,
                (frac_rect.min.y * MESH_CACHE_GRANULARITY) as u32,
                (frac_rect.max.x * MESH_CACHE_GRANULARITY) as u32,
                (frac_rect.max.y * MESH_CACHE_GRANULARITY) as u32,
            ];

            mesh_keys.push(mesh_key);

            // if we don't have a mesh in the cache, create it.
            if let Entry::Vacant(e) = params.sr.mesh_cache.entry(mesh_key) {
                let mut mesh = quad(w, h, Some(pivot), self.double_sided);
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    vec![
                        [frac_rect.min.x, frac_rect.max.y],
                        [frac_rect.max.x, frac_rect.max.y],
                        [frac_rect.min.x, frac_rect.min.y],
                        [frac_rect.max.x, frac_rect.min.y],
                        [frac_rect.min.x, frac_rect.max.y],
                        [frac_rect.max.x, frac_rect.max.y],
                        [frac_rect.min.x, frac_rect.min.y],
                        [frac_rect.max.x, frac_rect.min.y],
                    ],
                );
                let mesh_h = params.meshes.add(mesh);
                e.insert(mesh_h);
            }
        }

        return AtlasSprite3dBundle {
            pbr: PbrBundle {
                mesh: params
                    .sr
                    .mesh_cache
                    .get(&mesh_keys[self.index])
                    .unwrap()
                    .clone(),
                material: {
                    let mat_key = MatKey {
                        image: atlas.texture.clone(),
                        partial_alpha: self.partial_alpha,
                        unlit: self.unlit,
                        emissive: reduce_colour(self.emissive),
                    };
                    if let Some(material) = params.sr.material_cache.get(&mat_key) {
                        material.clone()
                    } else {
                        let material = params.materials.add(material(
                            atlas.texture.clone(),
                            self.partial_alpha,
                            self.unlit,
                            self.emissive,
                        ));
                        params.sr.material_cache.insert(mat_key, material.clone());
                        material
                    }
                },

                transform: self.transform,
                ..default()
            },
            params: AtlasSprite3dComponent {
                index: self.index,
                atlas: mesh_keys,
            },
        };
    }
}
