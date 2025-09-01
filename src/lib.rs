use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::hash_map::HashMap;
use bevy::prelude::*;
use bevy::render::{mesh::*, render_asset::RenderAssetUsages, render_resource::*};
use std::hash::Hash;

use bevy::{
    asset::WaitForAssetError,
    tasks::{block_on, poll_once, IoTaskPool, Task},
};

pub mod prelude;

pub struct Sprite3dPlugin;
impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprite3dCaches>()
            .init_resource::<WaitingSprites>()
            .add_systems(
                PostUpdate,
                (
                    finalize_waiting_sprites,
                    bundle_builder.after(finalize_waiting_sprites),
                    (handle_texture_atlases, handle_images).after(bundle_builder),
                ),
            );
    }
}

// sizes are multiplied by this, then cast to ints to query the mesh hashmap.
const MESH_CACHE_GRANULARITY: f32 = 1000.;

#[derive(Eq, Hash, PartialEq)]
pub struct MatKey {
    image: Handle<Image>,
    alpha_mode: HashableAlphaMode,
    unlit: bool,
    emissive: [u8; 4],
    base_color: [u8; 4],
    flip_x: bool,
    flip_y: bool,
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
            }
            AlphaMode::Blend => 2.hash(state),
            AlphaMode::Premultiplied => 3.hash(state),
            AlphaMode::Add => 4.hash(state),
            AlphaMode::Multiply => 5.hash(state),
            AlphaMode::AlphaToCoverage => 6.hash(state),
        }
    }
}

fn reduce_colour(c: LinearRgba) -> [u8; 4] {
    [
        (c.red * 255.) as u8,
        (c.green * 255.) as u8,
        (c.blue * 255.) as u8,
        (c.alpha * 255.) as u8,
    ]
}

fn reduce_color_from_bevy(c: Color) -> [u8; 4] {
    // Convert to linear first for consistent reduction
    let lin = c.to_linear();
    reduce_colour(lin)
}

#[derive(Resource, Default)]
pub struct Sprite3dCaches {
    pub mesh_cache: HashMap<[u32; 9], Mesh3d>,
    pub material_cache: HashMap<MatKey, MeshMaterial3d<StandardMaterial>>,
}

// resource holding tasks for entities waiting on assets
#[derive(Resource, Default)]
struct WaitingSprites(Vec<(Entity, Task<Result<(), WaitForAssetError>>)>);

// Marker: user supplied a material BEFORE Sprite3d inserted, so we preserve most fields and skip caching.
#[derive(Component)]
struct Sprite3dUserMaterial;

fn bundle_builder(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    mut caches: ResMut<Sprite3dCaches>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
    mut waiting: ResMut<WaitingSprites>,
    user_materials: Query<(), With<Sprite3dUserMaterial>>,
    mut query: Query<
        (
            &mut Sprite3d,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &Sprite,
            Entity,
        ),
        With<Sprite3dBuilder>,
    >,
) {
    // Entities without a Sprite won't match the query; once Sprite is added they will.
    for (mut sprite3d, mut mesh, mut mat, sprite, e) in query.iter_mut() {
        // check readiness
        let image_ready = images.get(&sprite.image).is_some();
        let (atlas_ready, layout_handle) = if let Some(atlas) = &sprite.texture_atlas {
            (
                atlas_layouts.get(&atlas.layout).is_some(),
                Some(atlas.layout.clone()),
            )
        } else {
            (true, None)
        };

        if !(image_ready && atlas_ready) {
            let image_h = sprite.image.clone();
            let layout_h = layout_handle;
            let server = asset_server.clone();
            let task = IoTaskPool::get().spawn(async move {
                server.wait_for_asset(&image_h).await?;
                if let Some(l) = layout_h {
                    server.wait_for_asset(&l).await?;
                }
                Ok(())
            });
            waiting.0.push((e, task));
            continue; // defer building until assets loaded
        }

        // get image dimensions
        let image_size = images.get(&sprite.image).unwrap().texture_descriptor.size;
        // w & h are the world-space size of the sprite.
        let w = (image_size.width as f32) / sprite3d.pixels_per_metre;
        let h = (image_size.height as f32) / sprite3d.pixels_per_metre;
        let pivot = sprite3d.pivot.unwrap_or(Vec2::new(0.5, 0.5));

        if let Some(atlas) = &sprite.texture_atlas {
            let atlas_layout = atlas_layouts.get(&atlas.layout).unwrap();

            // cache all the meshes for the atlas (if they haven't been already)
            // so that we can change the index later and not have to re-create the mesh.

            for i in 0..atlas_layout.textures.len() {
                let rect = atlas_layout.textures[i];

                let w = rect.width() as f32 / sprite3d.pixels_per_metre;
                let h = rect.height() as f32 / sprite3d.pixels_per_metre;

                let frac_rect = bevy::math::Rect {
                    min: Vec2::new(
                        rect.min.x as f32 / (image_size.width as f32),
                        rect.min.y as f32 / (image_size.height as f32),
                    ),

                    max: Vec2::new(
                        rect.max.x as f32 / (image_size.width as f32),
                        rect.max.y as f32 / (image_size.height as f32),
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
                    sprite3d.double_sided as u32,
                    (frac_rect.min.x * MESH_CACHE_GRANULARITY) as u32,
                    (frac_rect.min.y * MESH_CACHE_GRANULARITY) as u32,
                    (frac_rect.max.x * MESH_CACHE_GRANULARITY) as u32,
                    (frac_rect.max.y * MESH_CACHE_GRANULARITY) as u32,
                ];

                if sprite3d.texture_atlas_keys.last().copied() != Some(mesh_key) {
                    sprite3d.texture_atlas_keys.push(mesh_key);
                }

                // if we don't have a mesh in the cache, create it.
                if !caches.mesh_cache.contains_key(&mesh_key) {
                    let mut mesh = quad(w, h, Some(pivot), sprite3d.double_sided);
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
                    let mesh_h = Mesh3d(meshes.add(mesh));
                    caches.mesh_cache.insert(mesh_key, mesh_h);
                }
            }
        } else {
            // No texture atlas
            let mesh_key = [
                (w * MESH_CACHE_GRANULARITY) as u32,
                (h * MESH_CACHE_GRANULARITY) as u32,
                (pivot.x * MESH_CACHE_GRANULARITY) as u32,
                (pivot.y * MESH_CACHE_GRANULARITY) as u32,
                sprite3d.double_sided as u32,
                0,
                0,
                0,
                0,
            ];
            if sprite3d.texture_atlas_keys.last().copied() != Some(mesh_key) {
                sprite3d.texture_atlas_keys.push(mesh_key);
            }
        }

        *mesh = {
            let mesh_key = if let Some(atlas) = &sprite.texture_atlas {
                sprite3d.texture_atlas_keys[atlas.index]
            } else {
                *sprite3d.texture_atlas_keys.first().unwrap()
            };
            // if we have a mesh in the cache, use it.
            // (greatly reduces number of unique meshes for tilemaps, etc.)
            if let Some(mesh) = caches.mesh_cache.get(&mesh_key) {
                mesh.clone()
            } else {
                // otherwise, create a new mesh and cache it.
                let mesh = Mesh3d(meshes.add(quad(w, h, sprite3d.pivot, sprite3d.double_sided)));
                caches.mesh_cache.insert(mesh_key, mesh.clone());
                mesh
            }
        };

        // likewise for material, use the existing if the image is already cached.
        // (possibly look into a bool in Sprite3dBuilder to manually disable caching for an individual sprite?)
        // Material handling (hybrid): if user supplied a material prior to Sprite3d, preserve & modify minimal fields (skip caching).
        if user_materials.get(e).is_ok() {
            if let Some(existing) = materials.get_mut(&mat.0) {
                // Always enforce the sprite's texture (library controls texture).
                existing.base_color_texture = Some(sprite.image.clone());
                if sprite3d.alpha_mode != DEFAULT_ALPHA_MODE {
                    existing.alpha_mode = sprite3d.alpha_mode;
                }
                if sprite3d.unlit {
                    existing.unlit = true;
                }
                if sprite3d.emissive != LinearRgba::BLACK {
                    existing.emissive = sprite3d.emissive;
                }
                if sprite.flip_x || sprite.flip_y {
                    existing.flip(sprite.flip_x, sprite.flip_y);
                }
                // Preserve existing.base_color (user tint / colour authority).
            }
        } else {
            // Cached / new material path; base_color taken from Sprite.color for tint.
            let base_colour_arr = reduce_color_from_bevy(sprite.color);
            let mat_key = MatKey {
                image: sprite.image.clone(),
                alpha_mode: HashableAlphaMode(sprite3d.alpha_mode),
                unlit: sprite3d.unlit,
                emissive: reduce_colour(sprite3d.emissive),
                base_color: base_colour_arr,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
            };
            *mat = if let Some(material) = caches.material_cache.get(&mat_key) {
                material.clone()
            } else {
                let mut new_mat = build_material(
                    sprite.image.clone(),
                    sprite3d.alpha_mode,
                    sprite3d.unlit,
                    sprite3d.emissive,
                    sprite.flip_x,
                    sprite.flip_y,
                );
                // apply tint
                // Convert back to linear colour from reduced representation? We still have sprite.color.
                new_mat.base_color = sprite.color;
                let handle = MeshMaterial3d(materials.add(new_mat));
                caches.material_cache.insert(mat_key, handle.clone());
                handle
            };
        }

        commands.entity(e).remove::<Sprite3dBuilder>();
    }
}

// finalize deferred sprites once their tasks complete.
fn finalize_waiting_sprites(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    mut caches: ResMut<Sprite3dCaches>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut waiting: ResMut<WaitingSprites>,
    user_materials: Query<(), With<Sprite3dUserMaterial>>,
    mut query: Query<
        (
            &mut Sprite3d,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &Sprite,
            Entity,
        ),
        With<Sprite3dBuilder>,
    >,
) {
    let mut still_waiting = Vec::new();
    for (entity, task) in waiting.0.drain(..) {
        if !task.is_finished() {
            still_waiting.push((entity, task));
            continue;
        }
        match block_on(poll_once(task)) {
            Some(Ok(())) => {}
            Some(Err(e)) => {
                warn!("Sprite3d asset load failed: {e}");
                continue;
            }
            None => continue,
        }

        if let Ok((mut sprite3d, mut mesh, mut mat, sprite, e)) = query.get_mut(entity) {
            // Re-run construction body (assets guaranteed ready now).
            let image_size = if let Some(img) = images.get(&sprite.image) {
                img.texture_descriptor.size
            } else {
                continue;
            };
            let w = (image_size.width as f32) / sprite3d.pixels_per_metre;
            let h = (image_size.height as f32) / sprite3d.pixels_per_metre;
            let pivot = sprite3d.pivot.unwrap_or(Vec2::new(0.5, 0.5));

            if let Some(atlas) = &sprite.texture_atlas {
                if let Some(atlas_layout) = atlas_layouts.get(&atlas.layout) {
                    for rect in &atlas_layout.textures {
                        let w = rect.width() as f32 / sprite3d.pixels_per_metre;
                        let h = rect.height() as f32 / sprite3d.pixels_per_metre;
                        let frac_rect = bevy::math::Rect {
                            min: Vec2::new(
                                rect.min.x as f32 / (image_size.width as f32),
                                rect.min.y as f32 / (image_size.height as f32),
                            ),
                            max: Vec2::new(
                                rect.max.x as f32 / (image_size.width as f32),
                                rect.max.y as f32 / (image_size.height as f32),
                            ),
                        };
                        let mut rect_pivot = pivot;
                        rect_pivot.x *= frac_rect.width();
                        rect_pivot.y *= frac_rect.height();
                        rect_pivot += frac_rect.min;
                        let mesh_key = [
                            (w * MESH_CACHE_GRANULARITY) as u32,
                            (h * MESH_CACHE_GRANULARITY) as u32,
                            (rect_pivot.x * MESH_CACHE_GRANULARITY) as u32,
                            (rect_pivot.y * MESH_CACHE_GRANULARITY) as u32,
                            sprite3d.double_sided as u32,
                            (frac_rect.min.x * MESH_CACHE_GRANULARITY) as u32,
                            (frac_rect.min.y * MESH_CACHE_GRANULARITY) as u32,
                            (frac_rect.max.x * MESH_CACHE_GRANULARITY) as u32,
                            (frac_rect.max.y * MESH_CACHE_GRANULARITY) as u32,
                        ];
                        sprite3d.texture_atlas_keys.push(mesh_key);
                        if !caches.mesh_cache.contains_key(&mesh_key) {
                            let mut m = quad(w, h, Some(pivot), sprite3d.double_sided);
                            m.insert_attribute(
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
                            let mesh_h = Mesh3d(meshes.add(m));
                            caches.mesh_cache.insert(mesh_key, mesh_h);
                        }
                    }
                } else {
                    continue;
                }
            } else {
                let mesh_key = [
                    (w * MESH_CACHE_GRANULARITY) as u32,
                    (h * MESH_CACHE_GRANULARITY) as u32,
                    (pivot.x * MESH_CACHE_GRANULARITY) as u32,
                    (pivot.y * MESH_CACHE_GRANULARITY) as u32,
                    sprite3d.double_sided as u32,
                    0,
                    0,
                    0,
                    0,
                ];
                sprite3d.texture_atlas_keys.push(mesh_key);
            }

            *mesh = {
                let mesh_key = if let Some(atlas) = &sprite.texture_atlas {
                    sprite3d.texture_atlas_keys[atlas.index]
                } else {
                    *sprite3d.texture_atlas_keys.first().unwrap()
                };
                if let Some(m) = caches.mesh_cache.get(&mesh_key) {
                    m.clone()
                } else {
                    let m = Mesh3d(meshes.add(quad(w, h, sprite3d.pivot, sprite3d.double_sided)));
                    caches.mesh_cache.insert(mesh_key, m.clone());
                    m
                }
            };

            if user_materials.get(e).is_ok() {
                if let Some(existing) = materials.get_mut(&mat.0) {
                    existing.base_color_texture = Some(sprite.image.clone());
                    if sprite3d.alpha_mode != DEFAULT_ALPHA_MODE {
                        existing.alpha_mode = sprite3d.alpha_mode;
                    }
                    if sprite3d.unlit {
                        existing.unlit = true;
                    }
                    if sprite3d.emissive != LinearRgba::BLACK {
                        existing.emissive = sprite3d.emissive;
                    }
                    if sprite.flip_x || sprite.flip_y {
                        existing.flip(sprite.flip_x, sprite.flip_y);
                    }
                }
            } else {
                let base_colour_arr = reduce_color_from_bevy(sprite.color);
                let mat_key = MatKey {
                    image: sprite.image.clone(),
                    alpha_mode: HashableAlphaMode(sprite3d.alpha_mode),
                    unlit: sprite3d.unlit,
                    emissive: reduce_colour(sprite3d.emissive),
                    base_color: base_colour_arr,
                    flip_x: sprite.flip_x,
                    flip_y: sprite.flip_y,
                };
                *mat = if let Some(material) = caches.material_cache.get(&mat_key) {
                    material.clone()
                } else {
                    let mut new_mat = build_material(
                        sprite.image.clone(),
                        sprite3d.alpha_mode,
                        sprite3d.unlit,
                        sprite3d.emissive,
                        sprite.flip_x,
                        sprite.flip_y,
                    );
                    new_mat.base_color = sprite.color;
                    let handle = MeshMaterial3d(materials.add(new_mat));
                    caches.material_cache.insert(mat_key, handle.clone());
                    handle
                };
            }

            commands.entity(e).remove::<Sprite3dBuilder>();
        }
    }
    waiting.0 = still_waiting;
}

// Update the mesh when sprite image or atlas region changes
fn handle_images(
    mut caches: ResMut<Sprite3dCaches>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<
        (
            Entity,
            &mut MeshMaterial3d<StandardMaterial>,
            &Sprite,
            &Sprite3d,
            Option<&Sprite3dUserMaterial>,
        ),
        Changed<Sprite>,
    >,
) {
    for (entity, mut mesh_mat, sprite, sprite_3d, user_flag) in query.iter_mut() {
        if user_flag.is_some() {
            // Update only the controlled fields on the user's material; don't swap handle.
            if let Some(existing) = materials.get_mut(&mesh_mat.0) {
                existing.base_color_texture = Some(sprite.image.clone());
                if sprite_3d.alpha_mode != DEFAULT_ALPHA_MODE {
                    existing.alpha_mode = sprite_3d.alpha_mode;
                }
                if sprite_3d.unlit {
                    existing.unlit = true;
                }
                if sprite_3d.emissive != LinearRgba::BLACK {
                    existing.emissive = sprite_3d.emissive;
                }
                if sprite.flip_x || sprite.flip_y {
                    existing.flip(sprite.flip_x, sprite.flip_y);
                }
            } else {
                warn!(
                    "Sprite3d: user material asset for entity {:?} missing when updating image",
                    entity
                );
            }
            continue;
        }

        let mat_key = MatKey {
            image: sprite.image.clone(),
            alpha_mode: HashableAlphaMode(sprite_3d.alpha_mode),
            unlit: sprite_3d.unlit,
            emissive: reduce_colour(sprite_3d.emissive),
            base_color: reduce_color_from_bevy(sprite.color),
            flip_x: sprite.flip_x,
            flip_y: sprite.flip_y,
        };
        let mat = if let Some(material) = caches.material_cache.get(&mat_key) {
            material.clone()
        } else {
            let mut base = build_material(
                sprite.image.clone(),
                sprite_3d.alpha_mode,
                sprite_3d.unlit,
                sprite_3d.emissive,
                sprite.flip_x,
                sprite.flip_y,
            );
            base.base_color = sprite.color; // apply tint for new material path
            let material_h = MeshMaterial3d(materials.add(base));
            caches.material_cache.insert(mat_key, material_h.clone());
            material_h
        };

        if *mesh_mat != mat {
            *mesh_mat = mat;
        }
    }
}

// Update the mesh of a Sprite3d with an atlas sprite when its index changes.
fn handle_texture_atlases(
    caches: Res<Sprite3dCaches>,
    mut query: Query<(&mut Mesh3d, &Sprite3d, &Sprite), Changed<Sprite>>,
) {
    for (mut mesh, sprite_3d, sprite) in query.iter_mut() {
        let Some(texture_atlas) = &sprite.texture_atlas else {
            continue;
        };

        if let Some(key) = sprite_3d.texture_atlas_keys.get(texture_atlas.index) {
            if let Some(cached_mesh) = caches.mesh_cache.get(key) {
                *mesh = cached_mesh.clone();
            }
        }
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

    mesh.insert_indices(Indices::U32(if double_sided {
        vec![0, 1, 2, 1, 3, 2, 5, 4, 6, 7, 5, 6]
    } else {
        vec![0, 1, 2, 1, 3, 2]
    }));

    mesh
}

// generate a StandardMaterial useful for rendering a sprite
fn build_material(
    image: Handle<Image>,
    alpha_mode: AlphaMode,
    unlit: bool,
    emissive: LinearRgba,
    flip_x: bool,
    flip_y: bool,
) -> StandardMaterial {
    let mut mat = StandardMaterial {
        base_color_texture: Some(image),
        cull_mode: Some(Face::Back),
        alpha_mode,
        unlit,
        perceptual_roughness: 0.5,
        reflectance: 0.15,
        emissive,

        ..Default::default()
    };
    mat.flip(flip_x, flip_y);
    mat
}

#[derive(Component, Default)]
struct Sprite3dBuilder;

/// Represents a 3D sprite. May store texture atlas data -- note that modifying
/// `texture_atlas` and `texture_atlas_keys` on an already spawned sprite may
/// cause buggy behavior.
#[derive(Component)]
#[require(Transform, Mesh3d, Sprite3dBuilder)]
#[component(on_insert = sprite3d_on_insert)]
pub struct Sprite3d {
    pub texture_atlas_keys: Vec<[u32; 9]>,

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

// On-insert hook to detect user-provided material & handle missing Sprite component scenario.
fn sprite3d_on_insert(mut world: DeferredWorld, ctx: HookContext) {
    let entity = ctx.entity;
    if world
        .get::<MeshMaterial3d<StandardMaterial>>(entity)
        .is_some()
    {
        // User supplied a material before Sprite3d.
        world.commands().entity(entity).insert(Sprite3dUserMaterial);
    } else {
        // Insert a default placeholder material (texture & other fields set later).
        let handle = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        world
            .commands()
            .entity(entity)
            .insert(MeshMaterial3d(handle));
    }
}

impl Default for Sprite3d {
    fn default() -> Self {
        Self {
            texture_atlas_keys: Vec::new(),
            pixels_per_metre: 100.,
            pivot: None,
            alpha_mode: DEFAULT_ALPHA_MODE,
            unlit: false,
            double_sided: true,
            emissive: LinearRgba::BLACK,
        }
    }
}
