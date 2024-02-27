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


A more complicated scene: [`examples/dungeon.rs`](./examples/dungeon.rs). Try
this one with `cargo run --example dungeon`.

https://github.com/FraserLee/bevy_sprite3d/assets/30442265/1821b13c-9770-4f4e-a889-f67e06a3cda6



Some more examples. These don't use bevy, but demonstrate the effect style:

![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Crowd_01_compressed.png)

![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Shootout_01_compressed.png)

![hollow knight](https://imgur.com/jVWzh4i.png)

# Usage

Check out the [examples](./examples) for details. Tl;dr initialize the plugin with
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
as `bevy_sprite3d` uses some properties of the image (such as size and aspect
ratio) in constructing the 3d mesh. Examples show how to do this with Bevy's
`States`.

## Versioning

| `bevy_sprite3d` version | `bevy` version |
|-------------------------|----------------|
| 2.8                     | 0.13           |
| 2.7                     | 0.12           |
| 2.5 - 2.6               | 0.11           |
| 2.4                     | 0.10           |
| 2.1 - 2.3               | 0.9            |
| 1.1 - 2.0               | 0.8            |
| 1.0                     | 0.7            |

