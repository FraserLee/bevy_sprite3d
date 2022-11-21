#!/usr/bin/env python3

# super quick script to add padding between the tiles of a tileset, bleeding
# using the colour at the tile edges.

import sys
from PIL import Image

# get & process arguments

if len(sys.argv) < 4 or len(sys.argv) > 5:
    print("Usage: padding.py <tileset file> <tile width> <tile height> [padding (default 1px)]")
    sys.exit(1)

tileset = sys.argv[1]
tile_width = int(sys.argv[2])
tile_height = int(sys.argv[3])
padding = int(sys.argv[4]) if len(sys.argv) == 5 else 1

# load tileset, create some basic info
img = Image.open(tileset)
img_width, img_height = img.size

new_width = img_width + (img_width // tile_width) * padding * 2 # each tile gets padding on both sides
new_height = img_height + (img_height // tile_height) * padding * 2

new_img = Image.new("RGBA", (new_width, new_height), (0, 0, 0, 0))

# copy over the base existing tiles
for y in range(0, img_height, tile_height):
    for x in range(0, img_width, tile_width):
        tile = img.crop((x, y, x + tile_width, y + tile_height))
        new_img.paste(tile, (x + ((x // tile_width) * 2 + 1) * padding, y + ((y // tile_height) * 2 + 1) * padding))

# copy the left & right edges of the tiles over enough times to fill the padding gap.
for x in range(0, new_width, tile_width + padding * 2):
    for x1 in range(x + padding, x, -1):
        bar = new_img.crop((x1, 0, x1 + 1, new_height))
        new_img.paste(bar, (x1 - 1, 0))

for x in range(padding + tile_width - 1, new_width, tile_width + padding * 2):
    for x1 in range(x, x + padding):
        bar = new_img.crop((x1, 0, x1 + 1, new_height))
        new_img.paste(bar, (x1 + 1, 0))

# same for top & bottom
for y in range(0, new_height, tile_height + padding * 2):
    for y1 in range(y + padding, y, -1):
        bar = new_img.crop((0, y1, new_width, y1 + 1))
        new_img.paste(bar, (0, y1 - 1))

for y in range(padding + tile_height - 1, new_height, tile_height + padding * 2):
    for y1 in range(y, y + padding):
        bar = new_img.crop((0, y1, new_width, y1 + 1))
        new_img.paste(bar, (0, y1 + 1))

new_img.save(tileset[:-4] + "_padded.png")
