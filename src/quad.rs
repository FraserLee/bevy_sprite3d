use bevy::{
    prelude::*,
    render::{mesh::*, render_asset::RenderAssetUsages},
};

// creates a (potentially offset) quad mesh facing +z
// pivot has an expected range of p \in (0,0) to (1,1)
// (though you can go out of bounds without issue)
pub fn quad(w: f32, h: f32, pivot: Vec2, double_sided: bool) -> Mesh {
    // Set RenderAssetUsages to the default value. Maybe allow customization or
    // choose a better default?
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let vertices =  {
        let px = pivot.x * w;
        let py = pivot.y * h;
        vec![[-px, -py, 0.0], [w - px, -py, 0.0], [-px, h - py, 0.0], [w - px, h - py, 0.0],
             [-px, -py, 0.0], [w - px, -py, 0.0], [-px, h - py, 0.0], [w - px, h - py, 0.0]]
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0,  1.0], [0.0, 0.0,  1.0], [0.0, 0.0,  1.0], [0.0, 0.0,  1.0],
                                                       [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0]]);

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0],
                                                     [0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]]);

    mesh.insert_indices(Indices::U32(
        if double_sided { vec![0, 1, 2, 1, 3, 2, 5, 4, 6, 7, 5, 6] }
        else {            vec![0, 1, 2, 1, 3, 2] }
    ));

    mesh
}
