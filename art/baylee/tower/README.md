# Tower Echoes item portraits

Seven original fantasy inventory illustrations generated with the built-in
`image_gen` tool on 15 November 2026 at the owner's request. These are raster
illustrations with rendered 3D lighting, not meshes or real-time 3D models.
No reference images, third-party artwork, brand marks or card illustrations were
provided. The complete prompts and original generation paths are retained in
`portraits-generation.json`.

The selected PNGs ship unchanged in
`crates/baylee-client/assets/tower/portraits/`: heartwood, satchel, compass,
grove, healing, mana and mythic. Each is 1254 × 1254 with its generated alpha.
The originals remain in the generator's output directory too.

The client loads pictures on first use; Common through Mythic Rare share one
image per family. Rarity is expressed by the existing bronze/silver/gold/
mythic setting, engravings, corner stones and crown. The image shader preserves
the full picture within the setting and uses one texture sample. Frame animation
uses the GPU clock and follows reduced motion and ambient-quality preferences.
No image requests are made to Central, gateways or an image-generation service
while playing. Card offers keep the original procedural folio.

These pictures are distinct from permanent achievement medals. Adding a new
family requires its own original portrait and explicit catalog/client mapping;
an unrelated existing picture must not be silently reused.

The shared market now uses 84-pixel detail portraits, 56-pixel stock portraits,
32-pixel owned-slot portraits and 24-pixel charge/material markers, following
the shell's text/window scale. The compact inspector separates name and rarity
and keeps the effect, destination and purchase control together. These sizes
replace the earlier vertically stacked 128-pixel inspector after the native
market refinement passes. All appearances use the same cached texture/material.
`SHA256SUMS` is checked from the repository root.
