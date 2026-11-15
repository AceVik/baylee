# Tower Echoes item portraits

## Modular shop artwork (15 November 2026)

The owner-approved direction for the redesigned run shop is recorded in
`docs/proposals/tower-run-redesign.md`. Three new original preparation PNGs
are kept here: `shop-concept.png` (1536 × 1024), `shop-frame.png`
(1536 × 1024, alpha) and `shop-shelf.png` (2172 × 724, alpha).
The frame and shelf use the original generated concept as their only reference.
`shop-generation.json` retains the complete prompts, source paths and tool.
These assets are not yet wired into the native client. The intended renderer
preserves corners/endcaps, scales only rails and creates independent shelf rows;
shader ornament must obey reduced motion and use GPU time without idle writes.

## Inventory portraits

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

## Life-synergy portraits

Four additional original images were generated on 15 November 2026 for the next
catalog milestone: `spring-chalice`, `hearthstone`, `blood-amber` and `life-coil`.
Their complete prompts and original paths are in `life-portraits-generation.json`.
They use the same 1254 × 1254 generated alpha format, were visually inspected,
and were copied unchanged into the project's portrait directory. They have no
external reference art. The checksums below cover these images too.

Generation two maps all four pictures to their own admitted families and four
native rarity definitions each. They share the existing cached material and
reduced-motion behavior. Original on-table artifacts use the documented text
face, with their actual abilities and counters; the inventory illustration does
not borrow a Magic card identity. Rules and bounded shop calibration are in
`docs/proposals/tower-echoes-market.md`.
