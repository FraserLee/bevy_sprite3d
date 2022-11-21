# bevy_sprite3d
[![Crates.io](https://img.shields.io/crates/v/bevy_sprite3d.svg)](https://crates.io/crates/bevy_sprite3d)
[![MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./license.md)

Use 2d sprites in a 3d [bevy](https://bevyengine.org/) scene.

This is a pretty common setup in other engines (unity, godot, etc). Useful for:
- 2d games using bevy's lighting `(orthographic camera, 3d sprites)`
- 2d games with easier parallax and scale `(perspective camera, 3d sprites)`
- 2d games in a 3d world `(perspective camera, both 3d sprites and meshes)`
- 3d games with billboard sprites (a la
  [Delver](https://cdn.cloudflare.steamstatic.com/steam/apps/249630/ss_0187dc55d24155ca3944b4ccc827baf7832715a0.1920x1080.jpg))

Both meshes and materials are internally cached, so this crate can be used for
things like tilemaps without issue.

# Examples

Example using `bevy_sprite3d`:

![chaos](assets/example.gif)

Some more examples. These don't use bevy, but demonstrate the effect style:

![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Crowd_01_compressed.png)
![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Shootout_01_compressed.png)
![hollow knight](https://imgur.com/jVWzh4i.png)

# Usage

Check out the [examples](./examples) for more details. TLDR: initialize the plugin with
```rust
app.add_plugin(Sprite3dPlugin)
```
and spawn sprites with
```rust
fn setup(
    mut commands: Commands, 
    images: Res<ImageAssets>,
    mut sprite_params: Sprite3dParams
) {

    // ----------------------- Single Static Sprite ----------------------------

    commands.spawn_bundle(Sprite3d {
            image: images.sprite.clone(),

            pixels_per_metre: 400.,

            partial_alpha: true,

            unlit: true,

            ..default()

            // transform: Transform::from_xyz(0., 0., 0.),
            // pivot: Some(Vec2::new(0.5, 0.5)),
            // double_sided: true,

    }.bundle(&mut sprite_params));

    // ------------------- Texture Atlas (Sprite Sheet) ------------------------

    commands.spawn_bundle(AtlasSprite3d {
            atlas: images.sprite_sheet.clone(),

            pixels_per_metre: 32.,
            partial_alpha: true,
            unlit: true,

            index: 3,

            ..default()

            // transform: Transform::from_xyz(0., 0., 0.),
            // pivot: Some(Vec2::new(0.5, 0.5)),
            // double_sided: true,

    }.bundle(&mut sprite_params));
}
```

One small complication: your image assets should be loaded *prior* to spawning,
as `bevy_sprite3d` uses some properties of the image (such as size and aspect ratio)
in constructing the 3d mesh.

To that end, the examples use
[`bevy_asset_loader`](https://github.com/NiklasEi/bevy_asset_loader) for
simplicity. This is far from the only way to do it, but it provides a nice
template to get started.

## Versioning

| `bevy` version | `bevy_sprite3d` version |
| -------------- | ----------------------- |
| 0.9            | 2.2                     |
| 0.9            | 2.1                     |
| 0.8            | 2.0                     |
| 0.8            | 1.1                     |
| 0.7            | 1.0                     |





