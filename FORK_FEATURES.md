# Fork Features

This fork of [bevy_sprite3d](https://github.com/FraserLee/bevy_sprite3d) adds the following features on top of the original library:

## 1. Async Asset Loading (No Panic on Missing Assets)

The original library panics with `.unwrap()` if a sprite's image or texture atlas layout isn't loaded when the sprite is spawned. This fork defers sprite construction until assets are ready.

**How it works:**
- When a `Sprite3d` is spawned but its assets aren't loaded yet, the entity is added to a `WaitingSprites` resource
- An async task is spawned using `IoTaskPool` to wait for the asset to load
- The `finalize_waiting_sprites` system polls these tasks and completes sprite construction once assets are ready
- If asset loading fails, a warning is logged instead of panicking

**Benefits:**
- Sprites can be spawned at any time without pre-loading assets
- No need for complex state management to ensure assets are ready
- Graceful error handling with warnings instead of panics

## 2. User-Provided Material Support

The original library always creates and caches its own `StandardMaterial`. This fork detects when a user provides their own material and preserves it.

**How it works:**
- An `on_insert` component hook detects if a `MeshMaterial3d<StandardMaterial>` exists before `Sprite3d` is inserted
- If found, the entity is marked with `Sprite3dUserMaterial`
- For user materials, only texture-related fields are updated:
  - `base_color_texture` (always set to the sprite's image)
  - `alpha_mode`, `unlit`, `emissive` (only if non-default on `Sprite3d`)
  - `flip_x`, `flip_y` (if set on the `Sprite`)
- The user retains control over: `base_color`, `roughness`, `reflectance`, `metallic`, all texture maps, `cull_mode`, etc.

**Usage:**
```rust
commands.spawn((
    MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.5, 0.5),
        metallic: 0.8,
        ..default()
    })),
    Sprite3d::default(),
    Sprite {
        image: asset_server.load("sprite.png"),
        ..default()
    },
));
```

## 3. Sprite.color Tinting Support

The original library ignores the `Sprite.color` field. This fork uses it to tint cached materials.

**How it works:**
- The `base_color` is included in `MatKey` for material caching
- When creating a new cached material, `base_color` is set from `Sprite.color`
- Materials with different colors are cached separately

**Usage:**
```rust
commands.spawn((
    Sprite3d::default(),
    Sprite {
        image: asset_server.load("sprite.png"),
        color: Color::srgb(1.0, 0.0, 0.0), // Red tint
        ..default()
    },
));
```

**Note:** For user-provided materials, `Sprite.color` is ignored and you control `base_color` directly on your material.

---

## API Additions

### `Sprite3dUserMaterial` (Component)

A marker component automatically added when you provide a material before `Sprite3d`. You can query for this to detect which sprites use custom materials:

```rust
fn my_system(query: Query<Entity, With<Sprite3dUserMaterial>>) {
    for entity in query.iter() {
        // This entity has a user-provided material
    }
}
```

This component is exported in the prelude.
