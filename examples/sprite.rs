use bevy::prelude::*;
use bevy_sprite3d::prelude::*;

fn main()
{
    App::new().add_plugins((DefaultPlugins, Sprite3dPlugin)).add_systems(Startup, setup).run();
}

fn setup(mut commands: Commands)
{
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0., 5.)));

    // ----------------------- Spawn a 3D sprite -----------------------------

    commands.queue_spawn_scene(bsn! {
        Sprite { image: "icon.png" }
        Sprite3d { pixels_per_metre: 400., unlit: true }
        // Anchor::CENTER // default pivot
    });

    // -----------------------------------------------------------------------
}
