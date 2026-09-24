//! The keyword strip's twelve marks, baked out of the Mana font at startup.
//!
//! The marks used to be twelve procedural pictograms drawn in
//! `card_common.wgsl` — a wing, a skull, an eye that closed — each one
//! authored against the ten physical pixels a slot gets on a table card. They
//! are gone, and what is drawn in their place is the Mana font's own ability
//! glyph for each keyword: the picture a player already knows from every
//! other Magic interface they have used, which is a thing no drawing of ours
//! can be.
//!
//! # Why a distance field and not a picture
//!
//! `marks_strip` does not draw a mark, it consumes a *signed distance* to
//! one: the plate underneath, the halo (`exp(-d * 24)`), the ink ramp and the
//! pulse are all arithmetic on `d`. A coverage bitmap has an edge and no
//! distance, so it would keep the silhouette and lose everything around it.
//! So each glyph is rasterised large, run through an exact Euclidean distance
//! transform, and stored as a distance — [`RANGE`] cell units either side of
//! the outline, encoded into one byte with the outline at 0.5.
//!
//! # Why it is baked and not shipped
//!
//! `docs/legal.md` §2 keeps everything the table is made of computed rather
//! than downloaded, and there is no `textures/` directory to audit. An atlas
//! PNG committed beside the font would be the first one, for no gain: the
//! outlines are already in `assets/fonts/mana.ttf`, which the client loads
//! anyway to set mana costs. Baking costs twelve rasterisations once, off the
//! first frames, and keeps the font the single source of these shapes.
//!
//! # What this cannot do
//!
//! A sampled glyph cannot change *shape*, and the twelve pictograms did:
//! flying flapped, vigilance's eye closed, trample opened the ground under
//! itself. `card_common.wgsl` measured why that mattered — at ten pixels a
//! whole-glyph translation of a third of a pixel moves no pixel at all, while
//! a shape change turns pixels over. What survives here is the one whole-glyph
//! verb that also turns pixels over: a **scale** pulse, which moves the
//! silhouette by a pixel on every side at once. The rates and the rarity are
//! unchanged, and the table of them is still in the shader.

use baylee_client_core::cardcrest::{GLYPH_COUNT as CREST_COUNT, GLYPHS as CREST_GLYPHS};
use baylee_client_core::cardplate::{TEXT_ADV, TEXT_BASELINE, TEXT_CHARS, TEXT_EM, TEXT_RANGE};
use baylee_client_core::cardrail::{MARK_GLYPHS, MARK_ORDER};
use bevy::asset::RenderAssetUsages;
use bevy::asset::uuid_handle;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use swash::FontRef;
use swash::scale::image::Image as SwashImage;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::shape::ShapeContext;
use swash::zeno::Format;

/// The atlas every card material samples its marks from.
///
/// A fixed handle rather than a resource, because the four places a card
/// material is built have no business knowing about fonts, and because the
/// image has to exist *before* any of them: an `AsBindGroup` holding a
/// `Handle<Image>` whose bytes have not arrived fails to prepare, and a card
/// with no prepared material is not drawn at all. So the handle is filled
/// with a blank field at startup and rewritten in place when the font lands.
pub const MARKS: Handle<Image> = uuid_handle!("6e2f0c41-7b85-4b0f-9b2a-2f59b0f1ac31");

/// One mark's square in the atlas, in texels.
///
/// A mark is 8 physical pixels on a table card and several times that in the
/// preview, so this is generous on purpose: the distance is what is sampled,
/// and an undersized field rounds a thin stroke away before the shader ever
/// sees it.
pub const CELL: usize = 96;

/// How much of its cell a glyph is drawn into.
///
/// The rest is the margin the distance field needs: a glyph touching its cell
/// wall has no outside to measure into on that side, and the mark would lose
/// its halo along one edge.
const FILL: f32 = 0.74;

/// The supersampling factor the distance is measured at, before it is
/// averaged down to [`CELL`].
const SUPER: usize = 5;

/// How far either side of the outline the encoding reaches, in cell units.
///
/// It is wider than anything now reads — the halo is `exp(-d * 24)` and is
/// gone by 0.1 — and it stays that way because the outside of the field is
/// what the shader's clamp correction measures against past the cell wall,
/// and because halving it would buy precision at the edge that nobody can
/// see while dragging this comment and the wall encoding two tests check.
pub const RANGE: f32 = 0.25;

/// How many cells the marks take, at the head of the row.
pub const MARK_CELLS: usize = MARK_ORDER.len();

/// Where the corner's own alphabet starts.
pub const TEXT_BASE: usize = MARK_CELLS;

/// Where the identity column's three glyphs start.
pub const CREST_BASE: usize = TEXT_BASE + TEXT_CHARS.len();

/// The whole row: the strip's marks, the corner's characters, then the
/// identity column's three.
///
/// One texture and not three. The cells are the same square and the
/// sampling is the same arithmetic — only the bake rule and the encoded
/// range differ, and both of those are decided per cell at bake time. A
/// second atlas would be two more bindings in two materials, a second
/// handle the four material constructors have to know about, and a second
/// mirror test, for nothing.
///
/// The crest's three are baked by the *marks'* rule and out of the marks'
/// font, and they sit at the end of the row rather than beside their
/// cousins for one reason: a cell that moved would move every cell after
/// it, and the shader addresses the text by a base index.
pub const CELLS: usize = CREST_BASE + CREST_COUNT;

/// The atlas: one row of [`CELLS`] square cells.
#[must_use]
pub fn atlas_size() -> (u32, u32) {
    ((CELL * CELLS) as u32, CELL as u32)
}

/// An atlas with no marks in it — every texel "far outside".
///
/// This is what the handle holds until the font arrives, and what it keeps if
/// the font never does. A strip of empty plates is the honest failure: it says
/// the creature has keywords and does not lie about which.
#[must_use]
pub fn blank() -> Image {
    let (w, h) = atlas_size();
    let mut image = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8],
        TextureFormat::R8Unorm,
        // Both worlds, which is not what the rest of this client's generated
        // textures ask for. `textures.rs` writes `RENDER_WORLD` alone because
        // nothing ever reads those pixels again — and an image marked that
        // way has its bytes *taken out of* the main world the first time it
        // is extracted (`take_gpu_data`). This one is written a second time,
        // when the font finishes loading, so the main-world copy has to
        // survive until then. It costs 110 KB.
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        // Linear, because the whole point of a distance field is that it
        // interpolates; clamped, because a mark must never fetch its
        // neighbour's texel — the shader keeps inside the cell as well, and
        // this is the second lock.
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });
    image
}

/// Bakes the whole row: the strip's marks out of the Mana font, the corner's
/// alphabet out of the interface face.
///
/// `None` if the mark font is not one this can read — the atlas has no shape
/// at all then. A *text* font that cannot be read leaves its half blank and
/// says so, for the same reason a single missing glyph does: twelve marks and
/// no numerals is a worse client than no client, but it is still one.
#[must_use]
pub fn bake(marks: &[u8], text: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![0u8; CELL * CELL * CELLS];
    let font = FontRef::from_index(marks, 0)?;
    bake_symbols(&mut out, font, &symbol_cells());
    if let Some(font) = FontRef::from_index(text, 0) {
        bake_text(&mut out, font);
    }
    Some(out)
}

/// Every cell that comes out of the Mana font, and which glyph fills it.
///
/// The strip's twelve and the identity column's three, in one list because
/// they obey one rule — the alternative was a second copy of the
/// render-twice-and-normalise loop below, which is exactly the kind of
/// duplicate that drifts.
#[must_use]
pub fn symbol_cells() -> Vec<(usize, char)> {
    MARK_GLYPHS
        .iter()
        .enumerate()
        .map(|(i, g)| (i, *g))
        .chain(
            CREST_GLYPHS
                .iter()
                .enumerate()
                .map(|(i, g)| (CREST_BASE + i, *g)),
        )
        .collect()
}

/// A symbol cell each: normalised to one size and centred on its ink.
fn bake_symbols(out: &mut [u8], font: FontRef<'_>, cells: &[(usize, char)]) {
    let charmap = font.charmap();
    let mut context = ScaleContext::new();
    let side = CELL * SUPER;

    for (slot, glyph) in cells.iter().copied() {
        let id = charmap.map(glyph);
        if id == 0 {
            continue;
        }
        // Rendered twice, and the second one is what makes the row an
        // alphabet. A single em size for all twelve leaves them the sizes the
        // font drew them at, which are not one size: measured at `FILL`, the
        // ink boxes ran from defender's 0.48 × 0.66 of a cell to menace's
        // 0.74 × 0.70, so the tower and the hand stood visibly a size below
        // the wing and the skull. The first pass measures the ink, the second
        // asks for the size that makes its **longer** side exactly `FILL` —
        // longer and not the height, because menace is wider than it is tall
        // and normalising on height would push it through the margin the
        // distance field needs.
        let target = side as f32 * FILL;
        let Some(first) = render_glyph(&mut context, font, id, target) else {
            continue;
        };
        let longest = first.placement.width.max(first.placement.height);
        if longest == 0 {
            continue;
        }
        let rendered = if (longest as f32 - target).abs() < 1.0 {
            first
        } else {
            match render_glyph(&mut context, font, id, target * target / longest as f32) {
                Some(again) => again,
                None => first,
            }
        };
        let (gw, gh) = (
            rendered.placement.width as usize,
            rendered.placement.height as usize,
        );
        if gw == 0 || gh == 0 || gw > side || gh > side {
            continue;
        }

        // Centred on the *ink* rather than on the font's metrics: these
        // twelve glyphs come from different corners of a symbol font and
        // their sidebearings have nothing in common, so trusting the advance
        // would put the row out of step with itself by a pixel or two per
        // mark. What a player reads as centred is the picture.
        let mut mask = vec![false; side * side];
        let x0 = ((side - gw) / 2) as i32;
        let y0 = ((side - gh) / 2) as i32;
        stamp(&mut mask, side, &rendered.data, gw, gh, x0, y0);
        encode(out, slot, &signed_distance(&mask, side), side, RANGE);
    }
}

/// The corner's alphabet: one em size, one baseline, each glyph centred on
/// its own advance.
///
/// The exact opposite rule to the marks above, and the reason the two halves
/// are written out separately rather than parameterised. A row of symbols
/// wants every picture the same size, because nothing relates them but the
/// row. A row of *type* wants nothing of the kind: a `1` is narrower than an
/// `8` and a hyphen is half the height of a digit, and normalising either of
/// those away is how a font stops being a font. What a line of type is
/// aligned by is the baseline and the advance, so that is what is baked in.
fn bake_text(out: &mut [u8], font: FontRef<'_>) {
    let mut shape = ShapeContext::new();
    let mut scale = ScaleContext::new();
    let side = CELL * SUPER;
    let em = TEXT_EM * side as f32;
    let baseline = TEXT_BASELINE * side as f32;

    for (i, ch) in TEXT_CHARS.iter().enumerate() {
        let Some((id, _)) = shaped(&mut shape, font, em, *ch) else {
            continue;
        };
        let Some(rendered) = render_glyph(&mut scale, font, id, em) else {
            continue;
        };
        let (gw, gh) = (
            rendered.placement.width as usize,
            rendered.placement.height as usize,
        );
        if gw == 0 || gh == 0 || gw > side || gh > side {
            continue;
        }
        // The pen sits so that the glyph's *advance box* is centred in the
        // cell; the ink then lands wherever its sidebearings put it, which
        // for a slash is a little past both ends of that box and is correct.
        let pen = (side as f32 - TEXT_ADV[i] * side as f32) * 0.5;
        let mut mask = vec![false; side * side];
        stamp(
            &mut mask,
            side,
            &rendered.data,
            gw,
            gh,
            pen as i32 + rendered.placement.left,
            (baseline as i32) - rendered.placement.top,
        );
        encode(
            out,
            TEXT_BASE + i,
            &signed_distance(&mask, side),
            side,
            TEXT_RANGE,
        );
    }
}

/// Lays a rasterised glyph's coverage into the supersampled mask.
///
/// Clipped rather than trusted: a glyph whose sidebearing puts it past the
/// cell wall is a bake bug, and the alternative to clipping is an index that
/// wraps into the row above and draws a slice of the wrong character.
fn stamp(mask: &mut [bool], side: usize, data: &[u8], gw: usize, gh: usize, x0: i32, y0: i32) {
    for y in 0..gh {
        let ty = y0 + y as i32;
        if ty < 0 || ty >= side as i32 {
            continue;
        }
        for x in 0..gw {
            let tx = x0 + x as i32;
            if tx < 0 || tx >= side as i32 {
                continue;
            }
            if data[y * gw + x] >= 128 {
                mask[ty as usize * side + tx as usize] = true;
            }
        }
    }
}

/// Averages one cell's supersampled distance down and writes it as bytes.
///
/// `range` is per cell and not global: a mark is drawn with a halo that needs
/// field to burn off into, and a numeral is an edge and a fill. Spending a
/// text cell's margin on range it never reads would cost the digits size that
/// they do read, at eleven physical pixels on a table card.
fn encode(out: &mut [u8], slot: usize, field: &[f32], side: usize, range: f32) {
    let stride = CELL * CELLS;
    let block = (SUPER * SUPER) as f32;
    for y in 0..CELL {
        for x in 0..CELL {
            let mut sum = 0.0;
            for sy in 0..SUPER {
                for sx in 0..SUPER {
                    sum += field[(y * SUPER + sy) * side + x * SUPER + sx];
                }
            }
            // Pixels to cell units, then to a byte with the outline at the
            // middle of the range.
            let d = sum / block / side as f32;
            let v = (0.5 - d / (2.0 * range)).clamp(0.0, 1.0);
            out[y * stride + slot * CELL + x] = (v * 255.0).round() as u8;
        }
    }
}

/// The glyph a character shapes to, with lining tabular figures asked for.
///
/// Shaped rather than looked up in the character map, and that is the whole
/// of it: `AlegreyaSans-Bold`'s **default** digits are oldstyle, so a charmap
/// lookup returns a `3` that hangs a tenth of an em below the baseline and a
/// `6` that stands above the others — which on a power and toughness is
/// exactly the crooked, unaligned look this change exists to remove. `lnum`
/// asks for the lining set and `tnum` for the tabular one, and both are
/// needed: `tnum` alone selects this face's tabular *oldstyle* figures.
///
/// The advance comes back with it and is what
/// `the_advances_are_the_shipped_font_s_own` holds the mirrored table to.
fn shaped(
    context: &mut ShapeContext,
    font: FontRef<'_>,
    size: f32,
    ch: char,
) -> Option<(u16, f32)> {
    let mut shaper = context
        .builder(font)
        .size(size)
        .features([("lnum", 1u16), ("tnum", 1u16)])
        .build();
    let mut buffer = [0u8; 4];
    shaper.add_str(ch.encode_utf8(&mut buffer));
    let mut found = None;
    shaper.shape_with(|cluster| {
        if let (None, Some(glyph)) = (found, cluster.glyphs.first()) {
            found = Some((glyph.id, glyph.advance));
        }
    });
    found
}

/// One glyph, rasterised at an em size in pixels.
///
/// An embedded bitmap is accepted after an outline, because a symbol font is
/// allowed to carry one and a mark drawn from a strike is still a mark; what
/// it must not be is nothing.
fn render_glyph(
    context: &mut ScaleContext,
    font: FontRef<'_>,
    id: u16,
    size: f32,
) -> Option<SwashImage> {
    let mut scaler = context.builder(font).size(size).hint(false).build();
    Render::new(&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)])
        .format(Format::Alpha)
        .render(&mut scaler, id)
}

/// The symbols this font cannot draw, by codepoint.
///
/// A wrong codepoint renders as nothing at all, which on a card is an empty
/// plate and in a screenshot is easy to read as "that creature has no
/// keywords". Asking the font is the only way to know; the tables in
/// `cardrail` and `cardcrest` can only be checked for shape.
///
/// It answers codepoints rather than badge names because the identity
/// column's three have no badge — and a `char` out of the private-use block
/// is the one thing a reader can act on anyway, the fix being to look that
/// number up in the font's own stylesheet.
#[must_use]
pub fn missing(font: &[u8]) -> Vec<char> {
    let Some(font) = FontRef::from_index(font, 0) else {
        return symbol_cells().into_iter().map(|(_, g)| g).collect();
    };
    let charmap = font.charmap();
    symbol_cells()
        .into_iter()
        .map(|(_, g)| g)
        .filter(|g| charmap.map(*g) == 0)
        .collect()
}

/// Signed distance to the mask's outline, in pixels, negative inside.
///
/// Felzenszwalb and Huttenlocher's transform: two passes of an exact 1D
/// squared-distance transform over a lower envelope of parabolas, which is
/// linear in the number of pixels. The brute-force alternative is a windowed
/// search, and a window is a lie at exactly the distance that matters — it
/// reports the window's own radius for everything beyond it, so a mark's halo
/// would end in a ring.
fn signed_distance(mask: &[bool], side: usize) -> Vec<f32> {
    let inside = squared_distance(mask, side, false);
    let outside = squared_distance(mask, side, true);
    (0..side * side)
        .map(|i| {
            if mask[i] {
                -inside[i].sqrt()
            } else {
                outside[i].sqrt()
            }
        })
        .collect()
}

/// Squared distance to the nearest texel of the wanted kind.
fn squared_distance(mask: &[bool], side: usize, to_ink: bool) -> Vec<f32> {
    const FAR: f32 = 1e20;
    let mut grid: Vec<f32> = mask
        .iter()
        .map(|&ink| if ink == to_ink { 0.0 } else { FAR })
        .collect();

    let mut column = vec![0.0f32; side];
    for x in 0..side {
        for y in 0..side {
            column[y] = grid[y * side + x];
        }
        let done = envelope(&column);
        for y in 0..side {
            grid[y * side + x] = done[y];
        }
    }
    for y in 0..side {
        let row = envelope(&grid[y * side..(y + 1) * side]);
        grid[y * side..(y + 1) * side].copy_from_slice(&row);
    }
    grid
}

/// One dimension of it: the lower envelope of the parabolas `height[q] + (x-q)²`.
///
/// The names are the paper's — `v` the parabolas in the envelope, `z` the
/// boundaries between them, `k` the one being considered — because anybody
/// checking this against Felzenszwalb and Huttenlocher is reading their
/// figure 1 and not ours.
fn envelope(height: &[f32]) -> Vec<f32> {
    let n = height.len();
    let mut vertex = vec![0usize; n];
    let mut edge = vec![0.0f32; n + 1];
    let mut k = 0usize;
    edge[0] = f32::NEG_INFINITY;
    edge[1] = f32::INFINITY;
    for q in 1..n {
        loop {
            let p = vertex[k];
            let cross = ((height[q] + (q * q) as f32) - (height[p] + (p * p) as f32))
                / (2.0 * (q as f32 - p as f32));
            if cross <= edge[k] && k > 0 {
                k -= 1;
            } else {
                k += 1;
                vertex[k] = q;
                edge[k] = cross;
                edge[k + 1] = f32::INFINITY;
                break;
            }
        }
    }
    let mut out = vec![0.0f32; n];
    let mut k = 0usize;
    for (q, slot) in out.iter_mut().enumerate() {
        while edge[k + 1] < q as f32 {
            k += 1;
        }
        let p = vertex[k];
        let dx = q as f32 - p as f32;
        *slot = dx.mul_add(dx, height[p]);
    }
    out
}

/// Puts the blank atlas where the materials can find it, and bakes into it
/// when the font has loaded.
pub struct MarkAtlasPlugin;

impl Plugin for MarkAtlasPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, install)
            .add_systems(Update, bake_when_the_font_arrives);
    }
}

/// The blank field, before anything else can ask for the handle.
fn install(mut images: ResMut<Assets<Image>>) {
    // The only failure it can report is that this id is taken, and this id
    // is nobody else's: it is a `uuid_handle!` written once, in this file.
    let _ = images.insert(MARKS.id(), blank());
}

/// Bakes once, the first frame both font assets are readable.
///
/// A system rather than part of startup because the font is an asset: on the
/// web it is an HTTP fetch and there is no frame at which it is simply there.
/// Until then the strip draws its plates with nothing on them, which lasts a
/// frame or two and is what the blank atlas is for.
fn bake_when_the_font_arrives(
    fonts: Res<Assets<Font>>,
    ui: Option<Res<crate::hud::UiFonts>>,
    mut images: ResMut<Assets<Image>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(ui) = ui else { return };
    // Both, and not either: the row is baked once and written once, so a
    // pass made while the second font was still in flight would leave half
    // the atlas blank with the latch already set — a card corner with no
    // numerals on it for the rest of the session.
    let (Some(marks), Some(text)) = (fonts.get(&ui.mana), fonts.get(&ui.bold)) else {
        // The only "not yet" on this path, and the only reason the system
        // runs every frame. Everything below is terminal, one way or the
        // other, which is why the latch is set the moment the bytes exist:
        // a font that cannot be read this frame cannot be read next frame,
        // and rasterising a row of glyphs on every frame to find that out
        // again would be the expensive way to say nothing.
        return;
    };
    *done = true;
    let absent = missing(marks.data.data());
    if !absent.is_empty() {
        warn!("the mana font draws no glyph for: {absent:?}");
    }
    let Some(baked) = bake(marks.data.data(), text.data.data()) else {
        warn!("the mana font could not be read; the keyword marks will be blank");
        return;
    };
    let Some(mut image) = images.get_mut(&MARKS) else {
        // Not "not yet": `install` put it there at startup, so this can only
        // mean somebody took the handle out from under the materials, and
        // saying so beats a strip that is blank for the rest of the session.
        warn!("the mark atlas is gone from Assets<Image>; the keyword marks will be blank");
        return;
    };
    image.data = Some(baked);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shipped font, read the way the client reads it.
    fn font(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/assets/fonts/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap_or_else(|_| panic!("the bundled {name}"))
    }

    /// The whole row, baked out of both shipped files.
    fn row() -> Vec<u8> {
        bake(&font("mana.ttf"), &font("AlegreyaSans-Bold.ttf")).expect("the bundled fonts bake")
    }

    /// Every codepoint in `cardrail::MARK_GLYPHS` and `cardcrest::GLYPHS`
    /// is one this font actually draws.
    ///
    /// The tables cannot check this and neither can the compiler: a
    /// private-use codepoint off by one is a perfectly good `char` that
    /// renders as nothing, and the card it lands on shows an empty plate that
    /// reads as "no keyword". This is the only reader that has the file.
    #[test]
    fn the_shipped_font_draws_every_mark() {
        assert_eq!(missing(&font("mana.ttf")), Vec::<char>::new());
        // A population, not just a pass: a `symbol_cells` that answered
        // nothing would make the line above vacuous.
        assert_eq!(symbol_cells().len(), MARK_CELLS + CREST_COUNT);
    }

    /// Each symbol cell holds a picture: ink in the middle, nothing at the
    /// wall.
    ///
    /// Both halves are the finding. A cell that is blank everywhere is a
    /// glyph that did not rasterise, and a cell whose corners are inked is a
    /// glyph too big for its square — which is not a cosmetic complaint,
    /// because a mark with no outside along one edge has no distance to
    /// measure there and loses its halo.
    #[test]
    fn every_cell_is_inked_in_the_middle_and_clear_at_its_wall() {
        let baked = row();
        let stride = CELL * CELLS;
        assert_eq!(baked.len(), stride * CELL);
        let at = |slot: usize, x: usize, y: usize| baked[y * stride + slot * CELL + x];
        for (slot, mark) in symbol_cells() {
            let inside = (0..CELL)
                .flat_map(|y| (0..CELL).map(move |x| (x, y)))
                .filter(|(x, y)| at(slot, *x, *y) > 128)
                .count();
            assert!(
                inside > CELL * CELL / 50,
                "{mark:?} baked to {inside} inked texels, which is not a picture"
            );
            for k in 0..CELL {
                for (x, y) in [(0, k), (CELL - 1, k), (k, 0), (k, CELL - 1)] {
                    assert!(
                        at(slot, x, y) < 128,
                        "{mark:?} is inked at its cell wall ({x}, {y})"
                    );
                }
            }
        }
    }

    /// The field is a distance, not a silhouette.
    ///
    /// Walking across a mark, the encoded byte changes almost everywhere
    /// rather than switching between two values — which is what the halo and
    /// the one-pixel ink ramp both read. A coverage bitmap would pass the
    /// test above and fail this one flat: it is two values and has no slope.
    ///
    /// Both bounds are the measurement and not a guess, and each was set by
    /// the glyph that nearly failed it. Measured over the shipped font: the
    /// middle row changes at 66 of 96 texels (defender, the plainest
    /// silhouette) to 95 (first strike, deathtouch), and the deepest texel of
    /// a cell runs 158 (the two strikes, whose thickest stroke is a circle rim
    /// six texels wide) to 223 (defender).
    ///
    /// The depth is measured over the whole cell and not along that walk,
    /// which is the finding rather than a convenience. The row through the
    /// middle of defender's cell crosses the tower's *neck*, and the deepest
    /// point of a narrow neck is a tenth of a cell in — so a threshold that
    /// held for a heart or a skull failed on a glyph whose middle is its
    /// thinnest part.
    #[test]
    fn the_bytes_fall_away_from_the_ink_rather_than_switching() {
        let baked = row();
        let stride = CELL * CELLS;
        for (slot, mark) in symbol_cells() {
            let row: Vec<u8> = (0..CELL)
                .map(|x| baked[(CELL / 2) * stride + slot * CELL + x])
                .collect();
            let steps = row.windows(2).filter(|w| w[0] != w[1]).count();
            assert!(
                steps > CELL / 3,
                "{mark:?} changes at {steps} of {CELL} across its middle, which is a stencil"
            );
            // Outside at the wall, but not *saturated* there, and the
            // difference is a measurement rather than a nicety: a glyph
            // fills [`FILL`] of its cell, so the widest of them leaves 0.13
            // cell units of margin against a [`RANGE`] of 0.25 and its wall
            // encodes to about 61. That is why the shader adds back what its
            // clamp moved instead of trusting the field to have ended.
            assert!(
                row[0] < 128 && row[CELL - 1] < 128,
                "{mark:?} is inked at its cell wall"
            );
            let deepest = (0..CELL)
                .flat_map(|y| (0..CELL).map(move |x| (x, y)))
                .map(|(x, y)| baked[y * stride + slot * CELL + x])
                .max()
                .expect("a cell has texels");
            assert!(
                deepest > 150,
                "{mark:?} is nowhere deeper than {deepest} inside its own ink"
            );
        }
    }

    /// The symbols are one alphabet: every one of them is the same size on
    /// its longer side.
    ///
    /// This is the claim the second rasterisation in [`bake`] exists to make,
    /// and it is the one thing a reader cannot check by looking at the
    /// codepoints. Before it, the ink boxes ran from 0.48 × 0.66 of a cell
    /// (defender) to 0.74 × 0.70 (menace) and the row read as a font sample
    /// rather than as a row of marks.
    ///
    /// The tolerance is three texels of 96 and it is the measured population,
    /// not a guess: they come out 68 to 70 against a target of 71,
    /// because the size is asked for in fractional pixels and the box is then
    /// read back through two thresholds — the bake's `>= 128` coverage and
    /// this test's `> 128` distance. Small enough to fail the thing it is
    /// about: with the second pass removed, defender comes back 50 wide.
    #[test]
    fn every_mark_is_drawn_to_the_same_size() {
        let baked = row();
        let stride = CELL * CELLS;
        let want = (FILL * CELL as f32).round() as usize;
        for (slot, mark) in symbol_cells() {
            let inked: Vec<(usize, usize)> = (0..CELL)
                .flat_map(|y| (0..CELL).map(move |x| (x, y)))
                .filter(|(x, y)| baked[y * stride + slot * CELL + x] > 128)
                .collect();
            let w = inked.iter().map(|(x, _)| x).max().expect("ink")
                - inked.iter().map(|(x, _)| x).min().expect("ink")
                + 1;
            let h = inked.iter().map(|(_, y)| y).max().expect("ink")
                - inked.iter().map(|(_, y)| y).min().expect("ink")
                + 1;
            let longest = w.max(h);
            assert!(
                longest.abs_diff(want) <= 3,
                "{mark:?} is {w}×{h} where every mark should be {want} on its longer side"
            );
        }
    }

    /// The ink box of one cell, as `(x0, y0, x1, y1)` inclusive.
    fn ink(baked: &[u8], slot: usize) -> Option<(usize, usize, usize, usize)> {
        let stride = CELL * CELLS;
        let lit: Vec<(usize, usize)> = (0..CELL)
            .flat_map(|y| (0..CELL).map(move |x| (x, y)))
            .filter(|(x, y)| baked[y * stride + slot * CELL + x] > 128)
            .collect();
        let (first, _) = lit.split_first()?;
        let mut box_ = (first.0, first.1, first.0, first.1);
        for (x, y) in lit {
            box_.0 = box_.0.min(x);
            box_.1 = box_.1.min(y);
            box_.2 = box_.2.max(x);
            box_.3 = box_.3.max(y);
        }
        Some(box_)
    }

    /// The mirrored advance table is the shipped file's own.
    ///
    /// `cardplate::TEXT_ADV` is a hand-written table of font metrics, which
    /// is a second truth beside the file it was read out of — the shape this
    /// repository keeps turning into a generated table for exactly that
    /// reason. It cannot be generated here (the shader needs it as a
    /// compile-time constant and `baylee-client-core` cannot open an asset),
    /// so the file is asked instead, through the same shaper the bake uses
    /// and with the same two features on.
    #[test]
    fn the_advances_are_the_shipped_font_s_own() {
        let bytes = font("AlegreyaSans-Bold.ttf");
        let font = FontRef::from_index(&bytes, 0).expect("a font");
        let mut shape = ShapeContext::new();
        for (i, ch) in TEXT_CHARS.iter().enumerate() {
            let (id, advance) = shaped(&mut shape, font, 1000.0, *ch).expect("a shaped glyph");
            // Glyph 0 is `.notdef`, the box a face draws for a character it
            // does not have — inked and with an advance, so nothing below
            // would notice a character missing from the face.
            assert_ne!(id, 0, "{ch:?} is not in the face");
            let want = advance / 1000.0 * TEXT_EM;
            assert!(
                (TEXT_ADV[i] - want).abs() < 1e-4,
                "{ch:?} advances {want} and the table says {}",
                TEXT_ADV[i]
            );
        }
    }

    /// The digits are **lining** figures standing on one baseline.
    ///
    /// The failure this is here for is silent and is the default: asked for
    /// by character map, `AlegreyaSans-Bold` answers with its *oldstyle*
    /// figures, where `3`, `4`, `5`, `7` and `9` descend about a tenth of an
    /// em and `6` and `8` stand above the rest. A plate drawn from those
    /// looks precisely as crooked as the one this replaced, and no other
    /// test in this file would notice — the cells are inked, the field is a
    /// distance, the advances are tabular. Only the baked pixels say it.
    ///
    /// Bounds are the measured population: the ten feet sit within 2 texels
    /// of each other and the ten caps within 2, of a 96-texel cell. With the
    /// two features taken off `shaped`, the feet spread to 10.
    #[test]
    fn the_digits_are_lining_figures_on_one_baseline() {
        let baked = row();
        let boxes: Vec<(usize, usize, usize, usize)> = (0..10)
            .map(|d| ink(&baked, TEXT_BASE + d).unwrap_or_else(|| panic!("digit {d} is blank")))
            .collect();
        let feet: Vec<usize> = boxes.iter().map(|b| b.3).collect();
        let caps: Vec<usize> = boxes.iter().map(|b| b.1).collect();
        let spread = |v: &[usize]| {
            v.iter().max().copied().unwrap_or(0) - v.iter().min().copied().unwrap_or(0)
        };
        assert!(
            spread(&feet) <= 2,
            "the digits stand on {} different baselines: {feet:?}",
            spread(&feet)
        );
        assert!(
            spread(&caps) <= 2,
            "the digits are {} texels apart at the top: {caps:?}",
            spread(&caps)
        );
        // And they are centred in their cells, which is what
        // `TEXT_BASELINE` is computed to do.
        let mid = (feet[0] + caps[0]) as f32 * 0.5;
        assert!(
            (mid - CELL as f32 * 0.5).abs() <= 3.0,
            "a digit's middle is at {mid} of a {CELL}-texel cell"
        );
    }

    /// Every character the corner writes with is in the atlas, inside its
    /// own walls.
    ///
    /// The wall half is what the slash is here for: it is 0.795 em tall
    /// against a digit's 0.607, hung off a baseline placed by the digits, so
    /// it is the one glyph that can reach the cell's edge — and a glyph
    /// touching its wall has no outside left to measure a distance into.
    #[test]
    fn every_character_is_baked_and_clears_its_walls() {
        let baked = row();
        let margin = (TEXT_RANGE * CELL as f32).floor() as usize;
        for (i, ch) in TEXT_CHARS.iter().enumerate() {
            let (x0, y0, x1, y1) =
                ink(&baked, TEXT_BASE + i).unwrap_or_else(|| panic!("{ch:?} baked to nothing"));
            assert!(
                x0 >= margin && y0 >= margin && x1 < CELL - margin && y1 < CELL - margin,
                "{ch:?} inks ({x0},{y0})..({x1},{y1}) of a {CELL} cell with {margin} of margin"
            );
        }
    }

    /// The blank atlas is the size the shader indexes and is filled with
    /// "outside".
    #[test]
    fn the_blank_atlas_is_a_row_of_empty_cells() {
        let image = blank();
        let (w, h) = atlas_size();
        assert_eq!(image.texture_descriptor.size.width, w);
        assert_eq!(image.texture_descriptor.size.height, h);
        assert_eq!(image.texture_descriptor.format, TextureFormat::R8Unorm);
        assert!(
            image
                .data
                .as_ref()
                .is_some_and(|d| d.iter().all(|v| *v == 0))
        );
    }
}
