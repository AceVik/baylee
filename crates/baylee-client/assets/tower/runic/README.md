# Original runic equipment paintings

Generated 16 November 2026 with Codex's built-in image generation tool, requested
by the owner for Tower equipment. No third-party image or franchise reference
was supplied. Ten transparent PNG atlases each contain sixteen original objects;
rarity framing, highlights and motion are rendered by the existing item shader.
These are independent run equipment, not MTG cards or card illustrations.

Each source is 1254 × 1254 pixels, four columns and four rows. Row-major motifs
match `baylee_tower_relics::Sigil::ALL`: Dawn, Dusk, Root, Hearth, Bloom, Ash,
Ember, Lore, Pulse, Forge, Veil, Valor, Fang, Echo, Wraith, Star. Files match
`Vessel::ALL`. The shield and standard paintings have a transparent outer apron;
the client records their measured grid rectangles instead of cutting objects.
The accepted generated images retain their alpha unchanged. A shared texture serves all sixteen
objects and all four rarities; material handles are created only when seen.

## Prompt set

Shared prompt:

> Use case: stylized-concept. ONE production fantasy roguelike sprite atlas,
> exactly 4 columns by 4 rows of sixteen distinct OBJECTS, square canvas.
> Genuine transparent background; no floor, text, grid lines, frames or UI.
> Each object fully inside its equal cell, exactly centered, maximum 70% cell
> size with ample transparent margin. Consistent premium sculpted 3D fantasy
> game renders, clear readable silhouettes at 68px, warm highlights and cool
> rim lighting, original designs. Row-major motifs: 1 ivory/gold sunrise;
> 2 silver twilight moon; 3 wooden roots and emerald; 4 warm copper hearth;
> 5 pink flowers and green leaves; 6 blackened obsidian ash; 7 glowing orange
> ember; 8 sapphire scholarly runes; 9 crimson crystalline lifeblood;
> 10 hammered iron forge; 11 lavender veiled mist; 12 gold/cobalt heroic valor;
> 13 carved bone and fangs; 14 concentric turquoise echo rings; 15 spectral
> violet skull; 16 deep blue starfall. Uniform 4x4 grid is sampled directly
> by a shader. Every object has its own recognizable material, shape and
> motif. No copied franchise designs. Object kind:

Object suffixes: ornate chalices; short compact magical scepters; freestanding
faceted prisms; enchanted seed pods/sprouts/cradles; small closed magical tomes;
compact curled war horns; stout round magical elixir bottles; small broad
protective shields; compact ceremonial standards on short staffs; small opaque
enchanted metal mirrors with enamel motif faces and short handles.

For scepters, shields and banners the margin instruction was strengthened to
60% cell size, very wide transparent margin above and below, complete finial,
tip/base inside each cell. Lens regeneration explicitly prohibited see-through
glass, fog, glow, haze, aura and background shadows. Two cloudy lens drafts and
one clipped scepter draft were rejected and are not shipped. The second scepter
atlas failed the transparent-area check; a built-in `background-extraction` edit
preserved its sixteen objects and removed surrounding haze, smoke and glow.
Its prompt required alpha-zero gutters, opaque object surfaces, unchanged cell
order and complete silhouettes. No CLI/API fallback or programmatic image
postprocessing was used.

The project legal notice remains in place. The 16 November asset review is in
`docs/legal.md`; this provenance does not claim broader legal approval.
