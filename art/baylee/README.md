# Baylee · in sunlight

The project is dedicated to Viktor's much-loved cat Baylee, who died on
26 October 2026. The memorial shows her small, slender and curled up asleep
on a sunlit cloud, with the garden birds and pigeons she loved watching.
The world below suggests a future **Baylees World**: an ivory garden city,
bridges, a turquoise river, coast and mountains beneath a broad cloud horizon.
This is atmosphere for the existing login/lobby, not a playable open world.

The cloud portrait is the owner's explicitly approved fourth iteration. Its
PNG is unchanged: subsequent light, breathing and hover decoration are shader
layers. Both new logos include sleeping Baylee. No external game art was used.

| Runtime asset under `crates/baylee-client/assets/` | Master / preparation |
| --- | --- |
| `scenes/baylee-cloud.png` | Approved V4, 1536 × 1024 RGBA, SHA-256 in manifest |
| `scenes/baylees-world.png` | Generated city/world, 1672 × 941 |
| `brand/baylee-logo.png` | Generated transparent wordmark with Baylee, 1942 × 809 |
| `brand/baylee-logo-{48,96,192}.png` | Mechanical downsamples at the named heights |
| `brand/baylee-icon.png` | Generated sleeping Baylee icon, 1254 × 1254 |
| Native/browser icons | `baylee-tools installers art`; browser downsamples 32/180 px |
| `memorial/baylee-{1..5}.jpg` | Owner-supplied photographs, max 1400 px; location/date metadata removed |

[Exact image generation calls](memorial/prompts.md) preserve the iteration
history; [manifest](memorial/assets.json) identifies the selected outputs and
photo sources. The original photographs remain untouched in Pictures. Their
use in this local album was explicitly requested by the owner on 26 October.
The complete owner-approved remembrance is in the client’s German and English
Fluent files: eight chapters and 37 paragraphs, matching Baylee Central. The
“Bei uns / With us” date range is the time with the family, not an invented lifespan.

The interactive cloud has warm contour light, engraved botanical arcs, a small
sun ornament at the pointer, keyboard focus and an accessible button label.
The book supports independent photo/chapter navigation, a scrollable reading
column, Escape, a focus trap and return to its opener. Its header reuses the
approved cloud material; gold ornaments are geometry, independent of fonts. Reduced motion and low ambient quality freeze all idle movement.
The full-screen world uses one texture with restrained parallax, moving light
and cloud haze; it is a layered image with shader effects, not a 3D city.

[Verification](memorial/verification.md) records the live layout/input iterations,
the motion/still pixel comparison, audio measurements and automated checks.

The [previous sanctuary sources](sanctuary.md) remain as historical authoring
material. Current musical provenance is in [the memorial score](../music/memorial/README.md).
