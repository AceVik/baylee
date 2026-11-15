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

Shop and equipment details use a 128-pixel portrait; the repeated catalog tiles
stay at 84 pixels. Both use the same cached texture/material. The first real
render showed the 84-pixel detail image was too small, which led to this second
presentation pass. The 1280 × 720 purchase view retains its visible price,
destination and review button. `SHA256SUMS` is checked from the repository root.
