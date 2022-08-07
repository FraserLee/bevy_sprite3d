use bevy::prelude::*;
use bevy::render::{ mesh::*, render_resource::* };

use std::collections::HashMap;

pub struct Sprite3dPlugin;
impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprite3dRes>();
    }
}


// sizes are multiplied by this, then cast to ints to query the mesh hashmap.
const MESH_CACHE_GRANULARITY: f32 = 1000.;

use std::marker::PhantomData;
use bevy::ecs::system::SystemParam;

// everything needed to register a sprite, passed in one go.
#[derive(SystemParam)]
pub struct Sprite3dParams<'w, 's> {
    pub meshes    : ResMut<'w, Assets<Mesh>>,
    pub materials : ResMut<'w, Assets<StandardMaterial>>,
    pub images    : ResMut<'w, Assets<Image>>,
    pub atlases   : ResMut<'w, Assets<TextureAtlas>>,
    pub sr        : ResMut<'w, Sprite3dRes>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}

pub struct Sprite3dRes {
    pub mesh_cache: HashMap<[u32; 8], Handle<Mesh>>,
    pub material_cache: HashMap<(Handle<Image>, bool, bool), Handle<StandardMaterial>>,
}


impl Default for Sprite3dRes {
    fn default() -> Self {
        Sprite3dRes {
            mesh_cache: HashMap::new(),
            material_cache: HashMap::new(),
        }
    }
}





// creates a (potentially offset) quad mesh facing +z
// pivot = None will have a center pivot
// pivot = Some(p) will have an expected range of p \in (0,0) to (1,1)
// (though you can go out of bounds without issue)
fn quad(w: f32, h: f32, pivot: Option<Vec2>) -> Mesh {
    let w2 = w / 2.0;
    let h2 = h / 2.0;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
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

    mesh.set_indices(Some(Indices::U32(vec![0, 1, 2, 1, 3, 2,
                                            5, 4, 6, 7, 5, 6])));

    mesh
}




// generate a StandardMaterial useful for rendering a sprite
fn material(image: Handle<Image>, partial_alpha: bool, unlit: bool) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(image),
        cull_mode: Some(Face::Back),
        alpha_mode: if partial_alpha { AlphaMode::Blend } 
                    else { AlphaMode::Mask(0.5) },
        unlit,
        perceptual_roughness: 0.5,
        metallic: 0.4,
        reflectance: 0.15,

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

    /// the number of pixels per metre of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_metre: f32, 

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
}

impl Default for Sprite3d {    
    fn default() -> Self {
        Self {
            transform: Default::default(),
            image: Default::default(),
            pixels_per_metre: 100.,
            pivot: None,
            partial_alpha: false,
            unlit: false,
        }
    }
}

impl Sprite3d {

    /// creates a bundle of components from the Sprite3d struct.
    pub fn bundle(self, params: &mut Sprite3dParams ) -> PbrBundle {
        // get image dimensions
        let image_size = params.images.get(&self.image).unwrap().texture_descriptor.size;
        // w & h are the world-space size of the sprite.
        let w = (image_size.width  as f32) / self.pixels_per_metre;
        let h = (image_size.height as f32) / self.pixels_per_metre;


        return PbrBundle {
            mesh: {
                let pivot = self.pivot.unwrap_or(Vec2::new(0.5, 0.5));

                let hash_key = [(w * MESH_CACHE_GRANULARITY) as u32,
                                (h * MESH_CACHE_GRANULARITY) as u32,
                                (pivot.x * MESH_CACHE_GRANULARITY) as u32,
                                (pivot.y * MESH_CACHE_GRANULARITY) as u32,
                                0, 0, 0, 0
                                ];

                // if we have a mesh in the cache, use it.
                // (greatly reduces number of unique meshes for tilemaps, etc.)
                if let Some(mesh) = params.sr.mesh_cache.get(&hash_key) { mesh.clone() } 
                else { // otherwise, create a new mesh and cache it.
                    let mesh = params.meshes.add(quad( w, h, self.pivot ));
                    params.sr.mesh_cache.insert(hash_key, mesh.clone());
                    mesh
                }
            },


            // likewise for material, use the existing if the image is already cached.
            // (possibly look into a bool in Sprite3d to manually disable caching for an individual sprite?)
            material: {
                let mat_key = (self.image.clone(), self.partial_alpha, self.unlit);

                if let Some(material) = params.sr.material_cache.get(&mat_key) { material.clone() }
                else {
                    let material = params.materials.add(material(self.image.clone(), self.partial_alpha, self.unlit));
                    params.sr.material_cache.insert(mat_key, material.clone());
                    material
                }
            },

            transform: self.transform,

            ..default()
        }
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

    /// the number of pixels per metre of the sprite, assuming a `Transform::scale` of 1.0.
    pub pixels_per_metre: f32,

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
}

impl Default for AtlasSprite3d {
    fn default() -> AtlasSprite3d {
        AtlasSprite3d {
            transform: Transform::default(),
            atlas: Handle::<TextureAtlas>::default(),
            index: 0,
            pixels_per_metre: 100.,
            pivot: None,
            partial_alpha: false,
            unlit: false,
        }
    }
}


impl AtlasSprite3d {
    /// creates a bundle of components from the AtlasSprite3d struct.
    pub fn bundle(self, params: &mut Sprite3dParams ) -> PbrBundle {
        let atlas = params.atlases.get(&self.atlas).unwrap();
        let image = params.images.get(&atlas.texture).unwrap();
        let image_size = image.texture_descriptor.size;
        let rect = atlas.textures[self.index];

        let w = rect.width() / self.pixels_per_metre;
        let h = rect.height() / self.pixels_per_metre;

        let frac_rect = bevy::sprite::Rect {
            min: Vec2::new(rect.min.x / (image_size.width as f32),
                           rect.min.y / (image_size.height as f32)),

            max: Vec2::new(rect.max.x / (image_size.width as f32),
                           rect.max.y / (image_size.height as f32)),
        };

        return PbrBundle {
            mesh: {
                let mut pivot = self.pivot.unwrap_or(Vec2::new(0.5, 0.5));

                // scale pivot to be relative to the rect within the atlas.
                pivot.x *= frac_rect.width();
                pivot.y *= frac_rect.height();
                pivot += frac_rect.min;


                let hash_key = [(w * MESH_CACHE_GRANULARITY) as u32,
                                (h * MESH_CACHE_GRANULARITY) as u32,
                                (pivot.x * MESH_CACHE_GRANULARITY) as u32,
                                (pivot.y * MESH_CACHE_GRANULARITY) as u32,
                                (frac_rect.min.x * MESH_CACHE_GRANULARITY) as u32,
                                (frac_rect.min.y * MESH_CACHE_GRANULARITY) as u32,
                                (frac_rect.max.x * MESH_CACHE_GRANULARITY) as u32,
                                (frac_rect.max.y * MESH_CACHE_GRANULARITY) as u32];


                if let Some(mesh) = params.sr.mesh_cache.get(&hash_key) { mesh.clone() } 
                else {
                    let mut mesh = quad( w, h, self.pivot );
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
                    let mesh_h = params.meshes.add(mesh);
                    params.sr.mesh_cache.insert(hash_key, mesh_h.clone());
                    mesh_h
                }
            },

            material: {
                let mat_key = (atlas.texture.clone(), self.partial_alpha, self.unlit);
                if let Some(material) = params.sr.material_cache.get(&mat_key) { material.clone() } 
                else {
                    let material = params.materials.add(material(atlas.texture.clone(), self.partial_alpha, self.unlit));
                    params.sr.material_cache.insert(mat_key, material.clone());
                    material
                }
            },

            transform: self.transform,

            ..default()
        }
    }
}


