//! Integration tests for fork-specific features:
//! - Async asset loading (deferred construction)
//! - User-provided material preservation
//! - Sprite.color tinting support
//!
//! These tests verify the additional functionality added by this fork.

use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

/// Test that Sprite3dUserMaterial marker is added when user provides material first.
#[test]
fn user_material_detection()
{
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(AssetPlugin::default())
       .init_asset::<Image>()
       .init_asset::<Mesh>()
       .init_asset::<StandardMaterial>()
       .init_asset::<TextureAtlasLayout>()
       .add_plugins(Sprite3dPlugin);

    let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let user_mat =
        materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), ..default() });

    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let image = images.add(Image::default());

    // Spawn with user material BEFORE Sprite3d
    let entity =
        app.world_mut()
           .spawn((MeshMaterial3d(user_mat), Sprite3d::default(), Sprite { image, ..default() }))
           .id();

    // Run systems
    app.update();

    // Check that Sprite3dUserMaterial marker was added
    assert!(app.world().get::<Sprite3dUserMaterial>(entity).is_some(),
            "Sprite3dUserMaterial should be added when user provides material before Sprite3d");
}

/// Test that no Sprite3dUserMaterial marker is added when library creates material.
#[test]
fn no_user_material_when_library_creates()
{
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(AssetPlugin::default())
       .init_asset::<Image>()
       .init_asset::<Mesh>()
       .init_asset::<StandardMaterial>()
       .init_asset::<TextureAtlasLayout>()
       .add_plugins(Sprite3dPlugin);

    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let image = images.add(Image::default());

    // Spawn without user material
    let entity = app.world_mut().spawn((Sprite3d::default(), Sprite { image, ..default() })).id();

    // Run systems
    app.update();

    // Check that Sprite3dUserMaterial marker was NOT added
    assert!(app.world().get::<Sprite3dUserMaterial>(entity).is_none(),
            "Sprite3dUserMaterial should NOT be added when library creates material");
}

/// Test that MatKey includes base_color for proper caching of tinted sprites.
#[test]
fn sprite_color_creates_different_materials()
{
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(AssetPlugin::default())
       .init_asset::<Image>()
       .init_asset::<Mesh>()
       .init_asset::<StandardMaterial>()
       .init_asset::<TextureAtlasLayout>()
       .add_plugins(Sprite3dPlugin);

    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let image = images.add(Image::default());

    // Spawn two sprites with same image but different colors
    let entity1 = app.world_mut()
                     .spawn((Sprite3d::default(),
                             Sprite { image: image.clone(),
                                      color: Color::srgb(1.0, 0.0, 0.0), // Red
                                      ..default() }))
                     .id();

    let entity2 = app.world_mut()
                     .spawn((Sprite3d::default(),
                             Sprite { image,
                                      color: Color::srgb(0.0, 1.0, 0.0), // Green
                                      ..default() }))
                     .id();

    // Run systems
    app.update();

    // Get material handles
    let mat1 = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity1).map(|m| m.0.clone());
    let mat2 = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity2).map(|m| m.0.clone());

    assert!(mat1.is_some(), "Entity 1 should have a material");
    assert!(mat2.is_some(), "Entity 2 should have a material");

    // Materials should be different because colors are different
    assert_ne!(mat1.unwrap(),
               mat2.unwrap(),
               "Sprites with different colors should have different cached materials");
}

/// Test that sprites with unloaded assets don't panic (deferred loading).
#[test]
fn deferred_loading_no_panic()
{
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
       .add_plugins(AssetPlugin::default())
       .init_asset::<Image>()
       .init_asset::<Mesh>()
       .init_asset::<StandardMaterial>()
       .init_asset::<TextureAtlasLayout>()
       .add_plugins(Sprite3dPlugin);

    // Create an unloaded image handle (simulating asset not yet loaded)
    let unloaded_handle: Handle<Image> = Handle::default();

    // This should NOT panic - the sprite construction should be deferred
    let _entity = app.world_mut()
                     .spawn((Sprite3d::default(), Sprite { image: unloaded_handle, ..default() }))
                     .id();

    // Run systems - this would panic in the original implementation
    app.update();

    // If we get here without panicking, the test passes
}
