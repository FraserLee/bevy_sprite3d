# bevy_sprite3d

Use 2d sprites in a 3d scene. This was my go-to workflow back when I was using
Unity. This crate replicates it in [bevy](https://bevyengine.org/).

Useful for:
- 2d games using bevy's lighting `(orthographic camera, 3d sprites)`
- 2d games with easier parallax and scale `(perspective camera, 3d sprites)`
- 2d games in a 3d world `(perspective camera, both 3d sprites and meshes)`

You could also use this for billboard sprites in a 3d game (a la
[Delver](https://cdn.cloudflare.steamstatic.com/steam/apps/249630/ss_0187dc55d24155ca3944b4ccc827baf7832715a0.1920x1080.jpg)),
so long as you set the sprite rotation.


# Examples

Example using `bevy_sprite3d`:

![chaos](example.gif)

Some more examples. These don't use bevy, but demonstrate the effect style:

![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Crowd_01_compressed.png)
![the last night](https://cdn.cloudflare.steamstatic.com/steam/apps/612400/extras/TLN_Shootout_01_compressed.png)
![hollow knight](https://imgur.com/jVWzh4i.png)







