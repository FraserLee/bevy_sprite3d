use bevy::{
    prelude::*,
    render::render_resource::Face,
};

/// Returns a [StandardMaterial] with useful defaults for a 3D sprite.
pub fn material() -> StandardMaterial {
    StandardMaterial {
        cull_mode: Some(Face::Back),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: false,
        perceptual_roughness: 0.5,
        reflectance: 0.15,
        emissive: LinearRgba::BLACK,
        ..default()
    }
}
