use bevy::{
    asset::WaitForAssetError,
    ecs::{
        component::ComponentId,
        world::DeferredWorld,
    },
    prelude::*,
    tasks::{block_on, poll_once, IoTaskPool, Task}
};
use uuid::Uuid;

const DEFAULT_MATERIAL_ID: Uuid = Uuid::from_u128(0xb4c3caf5ead145b985d10d8a5fc676d5_u128);

pub mod prelude;
pub mod utils;

mod quad;

/// Holds the resources and systems necessary for a [Sprite3d] to work.
pub struct Sprite3dPlugin;

impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<WaitingForLoad>()
            .init_resource::<Assets<Billboard>>()
            .add_systems(Startup, default_material)
            .add_systems(PreUpdate, finish_billboards)
            .add_systems(PostUpdate, handle_texture_atlases);
    }
}

fn default_material(mut standard_materials: ResMut<Assets<StandardMaterial>>) {
    standard_materials.insert(
        AssetId::Uuid { uuid: DEFAULT_MATERIAL_ID },
        utils::material(),
    );
}

// Update the mesh of a Sprite3d with an texture atlas when its index changes.
fn handle_texture_atlases(
    billboards: Res<Assets<Billboard>>,
    mut query: Query<(&mut Mesh3d, &Sprite3d, &Sprite3dBillboard), Changed<Sprite3d>>,
) {
    for (mut mesh, sprite_3d, billboard_3d) in query.iter_mut() {
        let Some(texture_atlas) = &sprite_3d.texture_atlas else {
            continue;
        };

        let billboard = billboards.get(&billboard_3d.0).unwrap();
        let BillboardKind::Atlas { mesh_list, .. } = &billboard.kind else {
            panic!("attempted to apply TextureAtlas, but the billboard is not associated with one");
        };

        if !mesh_list.is_empty() {
            **mesh = mesh_list[texture_atlas.index].clone();
        }
    }
}

/// Represents a 3D sprite.
#[derive(Clone, Default, Component, Reflect)]
#[require(Sprite3dBillboard)]
pub struct Sprite3d {
    /// Holds texture atlas data for the sprite. The layout must match the corresponding
    /// layout stored in the [Billboard] at the risk of causing bugs or panics.
    pub texture_atlas: Option<TextureAtlas>,
}

impl Sprite3d {
    /// Create a new `Sprite3d`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<TextureAtlas> for Sprite3d {
    fn from(atlas: TextureAtlas) -> Self {
        Self {
            texture_atlas: Some(atlas),
        }
    }
}

// Defines whether the billboard stores a single image or a texture atlas.
#[derive(Clone)]
enum BillboardKind {
    Single {
        mesh: Handle<Mesh>,
    },
    Atlas {
        mesh_list: Vec<Handle<Mesh>>,
        layout: Handle<TextureAtlasLayout>,
    }
}

// stores entities whose billboards are potentially waiting for their image
// to load
#[derive(Resource, Deref, DerefMut, Default)]
struct WaitingForLoad(Vec<(Entity, Task<Result<(), WaitForAssetError>>)>);

#[derive(Clone, Component, Deref, Default)]
#[require(Transform, Mesh3d, MeshMaterial3d<StandardMaterial>(set_material))]
#[component(on_insert = add_to_waiting_list)]
/// Holds the [Billboard] associated with a [Sprite3d]. Has no effect if inserted
/// into an entity without the `Sprite3d` component. The inner `Handle<Billboard>` is
/// private to prevent direct modification, but can be read through dereference.
///
/// An internal system will update the [Mesh3d] and [MeshMaterial3d] components after
/// this component is inserted, once the [Image] associated with the `Billboard` is
/// fully loaded.
pub struct Sprite3dBillboard(Handle<Billboard>);

impl Sprite3dBillboard {
    /// Create a new `Sprite3dBillboard`.
    pub fn new(billboard: Handle<Billboard>) -> Self {
        Self::from(billboard)
    }
}

impl From<Handle<Billboard>> for Sprite3dBillboard {
    fn from(handle: Handle<Billboard>) -> Self {
        Self(handle)
    }
}

fn set_material() -> MeshMaterial3d<StandardMaterial> {
    MeshMaterial3d(Handle::Weak(AssetId::Uuid {
        uuid: DEFAULT_MATERIAL_ID,
    }))
}

/// Represents the "billboard", a flat rectangular 3D mesh that the sprite is
/// displayed on. Attached onto a sprite with the [Sprite3dBillboard] component.
#[derive(Clone, Asset, TypePath)]
pub struct Billboard {
    // The image associated with the billboard.
    #[dependency]
    image: Handle<Image>,
    // See [BillboardKind].
    kind: BillboardKind,
    pixels_per_metre: f32,
    pivot: Vec2,
    double_sided: bool,
    rendered: bool,
}

impl Billboard {
    /// Creates a billboard associated with a single image.
    ///
    /// * `image`: The handle to the image associated with this billboard,
    ///   loaded or not.
    /// * `pixels_per_metre`: The number of pixels per metre of the sprite,
    ///   assuming a `Transform::scale` of `1.0`. Defaults to `100.0`.
    /// * `pivot`: The point around which the sprite will rotate. Defaults to the center
    ///   of the image, `Vec2(0.5, 0.5)`.
    /// * `double_sided`: Whether the billboard displays the image on both sides or
    ///   just the 'front'. Defaults to `true`.
    pub fn new(
        image: Handle<Image>,
        pixels_per_metre: f32,
        pivot: Option<Vec2>,
        double_sided: bool,
    ) -> Self {
        Self {
            image,
            pixels_per_metre,
            pivot: pivot.unwrap_or(Vec2::splat(0.5)),
            double_sided,
            ..default()
        }
    }

    /// Creates a billboard associated with a texture atlas. Refer to [Billboard::new]
    /// for more details.
    pub fn with_texture_atlas(
        image: Handle<Image>,
        layout: Handle<TextureAtlasLayout>,
        pixels_per_metre: f32,
        pivot: Option<Vec2>,
        double_sided: bool,
    ) -> Self {
        Self {
            image,
            kind: BillboardKind::Atlas {
                mesh_list: Vec::new(),
                layout,
            },
            pixels_per_metre,
            pivot: pivot.unwrap_or(Vec2::splat(0.5)),
            double_sided,
            ..default()
        }
    }
}

impl Default for Billboard {
    fn default() -> Self {
        Self {
            image: Default::default(),
            kind: BillboardKind::Single {
                mesh: Default::default(),
            },
            pixels_per_metre: 100.,
            pivot: Vec2::splat(0.5),
            double_sided: true,
            rendered: false,
        }
    }
}

impl From<Handle<Image>> for Billboard {
    fn from(image: Handle<Image>) -> Self {
        Self {
            image,
            ..default()
        }
    }
}

impl From<(Handle<Image>, Handle<TextureAtlasLayout>)> for Billboard {
    fn from((image, layout): (Handle<Image>, Handle<TextureAtlasLayout>)) -> Self {
        Self {
            image,
            kind: BillboardKind::Atlas {
                mesh_list: Vec::new(),
                layout,
            },
            ..default()
        }
    }
}

// Creates a task to finish constructing the billboard after the image loads,
// if it hasn't already been loaded.
fn add_to_waiting_list(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let task_pool = IoTaskPool::get();
    let asset_server = world.resource::<AssetServer>().clone();
    let billboard_h = world.get::<Sprite3dBillboard>(entity).unwrap();
    let billboard = world.resource::<Assets<Billboard>>().get(&**billboard_h).unwrap();
    let image_h = billboard.image.clone();

    let task = task_pool.spawn(async move {
        asset_server.wait_for_asset(&image_h).await
    });

    world.resource_mut::<WaitingForLoad>().push((entity, task));
}


// Finishes rendering the `Billboard` and transferring the associated data to the entity
// as each image in `WaitingForLoad` finishes loading.
#[allow(clippy::too_many_arguments)]
fn finish_billboards(
    images: Res<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    mut billboards: ResMut<Assets<Billboard>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut waiting_list: ResMut<WaitingForLoad>,
    mut sprite_query: Query<(
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        &Sprite3dBillboard,
        &Sprite3d,
    )>,
) {
    let mut still_waiting = Vec::new();

    for (entity, task) in waiting_list.drain(..) {
        // If the image is still unloaded, defer until later.
        if !task.is_finished() {
            still_waiting.push((entity, task));
            continue;
        }

        match block_on(poll_once(task)).unwrap() {
            Ok(()) => (),
            Err(asset_error) => {
                // should this be a logged error instead?
                panic!(
                    "Failed to load image while making bevy_sprite3d::Billboard: {}",
                    asset_error,
                );
            },
        }

        let (mut mesh_3d, mut material_3d, billboard_3d, sprite_3d) =
            sprite_query.get_mut(entity).unwrap();
        let billboard = billboards.get_mut(&billboard_3d.0).unwrap();

        // If the `Billboard` has not yet had its associated mesh(es) created,
        // then we do so here. This prevents unneceessary work if the same
        // `Billboard` is used multiple times.
        if !billboard.rendered {
            let image = images.get(&billboard.image).unwrap();
            let image_size = image.texture_descriptor.size;

            match &mut billboard.kind {
                BillboardKind::Single { mesh } => {
                    // w & h are the world-space size of the sprite
                    let w = (image_size.width as f32) / billboard.pixels_per_metre;
                    let h = (image_size.height as f32) / billboard.pixels_per_metre;

                    let new_mesh =
                        quad::quad(w, h, billboard.pivot, billboard.double_sided);
                    let mesh_handle = meshes.add(new_mesh.clone());
                    *mesh = mesh_handle;
                },
                BillboardKind::Atlas { mesh_list, layout } => {
                    let layout = layouts.get(layout).unwrap();
                    *mesh_list = layout.textures.iter().map(|rect| {
                        let w = rect.width() as f32 / billboard.pixels_per_metre;
                        let h = rect.height() as f32 / billboard.pixels_per_metre;

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

                        // scale pivot to be relative to the rect within the atlas
                        let mut rect_pivot = billboard.pivot;
                        rect_pivot.x *= frac_rect.width();
                        rect_pivot.y *= frac_rect.height();
                        rect_pivot += frac_rect.min;

                        let mut mesh =
                            quad::quad(w, h, billboard.pivot, billboard.double_sided);
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
                            ]
                        );

                        meshes.add(mesh)
                    }).collect();
                },
            }

            billboard.rendered = true;
        }

        // Replace the `Handle<Mesh>` stored in `Mesh3d` with the one
        // stored in the `Billboard`.
        match &billboard.kind {
            BillboardKind::Single { mesh } => {
                **mesh_3d = mesh.clone();
            },
            BillboardKind::Atlas { mesh_list, .. } => {
                let atlas = sprite_3d.texture_atlas
                    .as_ref()
                    .expect("missing texture atlas in Sprite3d");
                **mesh_3d = mesh_list[atlas.index].clone();
            },
        }

        // Create a copy of the `StandardMaterial` associated with the entity,
        // attach the image stored in the `Billboard`, then replace the handle
        // stored in the `MeshMaterial3d`.
        let mut new_material = materials.get_mut(&**material_3d).unwrap().clone();
        new_material.base_color_texture = Some(billboard.image.clone());
        **material_3d = materials.add(new_material);
    }

    **waiting_list = still_waiting;
}
