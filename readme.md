# bevy_sprite3d
[![Crates.io](https://img.shields.io/crates/v/bevy_sprite3d.svg)](https://crates.io/crates/bevy_sprite3d)
[![MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./license.md)

Use 2d sprites in a 3d scene. This was my go-to workflow back when I was using
Unity. This crate replicates it in [bevy](https://bevyengine.org/).

Useful for:
- 2d games using bevy's lighting `(orthographic camera, 3d sprites)`
- 2d games with easier parallax and scale `(perspective camera, 3d sprites)`
- 2d games in a 3d world `(perspective camera, both 3d sprites and meshes)`
- 3d games with billboard sprites (a la
  [Delver](https://cdn.cloudflare.steamstatic.com/steam/apps/249630/ss_0187dc55d24155ca3944b4ccc827baf7832715a0.1920x1080.jpg))


Both meshes and materials are internally cached, so you can use this for things
like tilemaps without issue.

# Examples

Example using `bevy_sprite3d`:

![chaos](example.gif)

Some more examples. These don't use bevy, but demonstrate the effect style:

![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Crowd_01_compressed.png)
![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Shootout_01_compressed.png)
![hollow knight](https://imgur.com/jVWzh4i.png)

# Usage

One small complication to `bevy_sprite3d` is that your image assets need to be
loaded *prior* to spawning, as the crate uses some properties of the image
(such as size and aspect ratio) in constructing the 3d mesh.

The following examples will use
[`bevy_asset_loader`](https://github.com/NiklasEi/bevy_asset_loader) for
simplicity. Even still, there's a fair amount of boilerplate due to this
loading-before-spawning requirement. If anyone knows a simpler way to write the
examples, please update it!


## Single Sprite

```rust
use bevy::prelude::*;
use bevy_sprite3d::*;
use bevy_asset_loader::prelude::*;


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum GameState { Loading, Ready }

#[derive(AssetCollection)]
struct ImageAssets {
    #[asset(path = "branding/icon.png")]
    icon: Handle<Image>,
}

fn main() {

    App::new()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Ready)
                .with_collection::<ImageAssets>()
        )
        .add_state(GameState::Loading)
        .add_plugins(DefaultPlugins)
        .add_plugin(Sprite3dPlugin)
        .add_system_set( SystemSet::on_enter(GameState::Ready).with_system(setup) )
        .run();

}


fn setup(
    mut commands: Commands, 
    images: Res<ImageAssets>,
    mut sprite_params: Sprite3dParams
) {

    commands.spawn_bundle(Camera3dBundle::default())
            .insert(Transform::from_xyz(0., 0., 5.));

    // ----------------------- Spawn a 3D sprite -----------------------------

    commands.spawn_bundle(Sprite3d {
            image: images.icon.clone(),

            pixels_per_metre: 400.,

            partial_alpha: true,

            unlit: true,

            // transform: Transform::from_xyz(0., 0., 0.),
            // pivot: Some(Vec2::new(0.5, 0.5)),

            ..default()
    }.bundle(&mut sprite_params));

    // -----------------------------------------------------------------------
}
```

## Sprite Sheet

```rust
use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use bevy_sprite3d::*;
use bevy_asset_loader::prelude::*;


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum GameState { Loading, Ready }

#[derive(AssetCollection)]
struct ImageAssets {
    #[asset(texture_atlas(tile_size_x = 24., tile_size_y = 24.))]
    #[asset(texture_atlas(columns = 7, rows = 1))]
    #[asset(path = "textures/rpg/chars/gabe/gabe-idle-run.png")]
    run: Handle<TextureAtlas>,
}

fn main() {

    App::new()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Ready)
                .with_collection::<ImageAssets>()
        )
        .insert_resource(ImageSettings::default_nearest())
        .add_state(GameState::Loading)
        .add_plugins(DefaultPlugins)
        .add_plugin(Sprite3dPlugin)
        .add_system_set( SystemSet::on_enter(GameState::Ready).with_system(setup) )
        .run();

}


fn setup(
    mut commands: Commands, 
    images: Res<ImageAssets>,
    mut sprite_params: Sprite3dParams
) {

    commands.spawn_bundle(Camera3dBundle::default())
            .insert(Transform::from_xyz(0., 0., 5.));

    // -------------------- Spawn a 3D atlas sprite --------------------------

    commands.spawn_bundle(AtlasSprite3d {
            atlas: images.run.clone(),

            pixels_per_metre: 32.,
            partial_alpha: true,
            unlit: true,

            index: 3,

            // transform: Transform::from_xyz(0., 0., 0.),
            // pivot: Some(Vec2::new(0.5, 0.5)),

            ..default()
    }.bundle(&mut sprite_params));

    // -----------------------------------------------------------------------
}

```

## Versioning

| `bevy` version | `bevy_sprite3d` version |
| -------------- | ----------------------- |
| 0.7            | 1.0                     |
| 0.8            | 1.1                     |




