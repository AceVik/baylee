use bevy::prelude::Color;

/// Panel background.
pub const PANEL: Color = Color::srgba(0.05, 0.06, 0.08, 0.88);
/// Slightly lighter panel (active tab, tooltip).
pub const PANEL_LIT: Color = Color::srgba(0.10, 0.13, 0.16, 0.94);
/// [`PANEL_LIT`] with the pointer on it.
///
/// One step and no more: the stack's top row is already the loud thing in
/// its panel, and a hover that also brightened its ink or its rail would
/// be answering a second time — the card preview the same hover opens is
/// the real answer. Lifted in all three channels rather than in alpha,
/// because this row is the one that rests at almost full opacity and has
/// nowhere left to go there.
pub const PANEL_HOT: Color = Color::srgba(0.13, 0.17, 0.20, 0.94);
/// Primary text.
pub const INK: Color = Color::srgb(0.90, 0.93, 0.94);
/// Secondary text.
pub const MUTED: Color = Color::srgb(0.58, 0.64, 0.68);

/// Translucent blue-green glass carrying the garden's lobby/editor panels.
pub const SANCTUARY_PANEL: Color = Color::srgba(0.035, 0.075, 0.105, 0.88);
/// A seat that has lost.
pub const DEAD: Color = Color::srgb(0.30, 0.32, 0.34);
/// The accent used for anything asking for a decision: the dial's
/// priority hand (`dial::PRIORITY_INK`, `felt.wesl`'s `TEAL`).
pub const ACCENT: Color = {
    let [r, g, b] = baylee_client_core::dial::PRIORITY_INK;
    Color::srgb(r, g, b)
};
/// Danger: lethal damage, a seat about to lose.
pub const DANGER: Color = Color::srgb(0.91, 0.47, 0.42);
/// Life gained — [`DANGER`]'s answer, and the only green on this
/// overlay.
///
/// Leaf rather than emerald, which is what keeps it away from [`ACCENT`]:
/// the accent is a cyan-leaning teal and means *this is asking you
/// something*, and a life gain is asking nothing. Two greens that had to
/// be told apart would be a third claim the player never agreed to read.
pub const HEAL: Color = Color::srgb(0.53, 0.80, 0.44);
/// The active seat's marker.
pub const ACTIVE: Color = Color::srgb(0.84, 0.64, 0.31);
/// A card the client is offering to tap lands for: an offer, not a
/// legal action, and drawn as the weaker claim it is.
pub const REACHABLE: Color = Color::srgb(0.50, 0.47, 0.84);
/// The fill behind selected text in a field.
///
/// [`ACCENT`] at a quarter, because the same accent rings the field that
/// has the caret: a selection and a focus are the same claim about the
/// same box — *this is where the typing goes* — and saying it in two
/// colours would make them two facts.
pub const SELECTION: Color = Color::srgba(0.33, 0.75, 0.71, 0.25);
/// Soft shadow under raised elements.
pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

/// Midnight mineral leather, shared by the hand dock and seat furniture.
pub const DOCK_GROUND: Color = Color::srgb(0.055, 0.075, 0.090);
/// Aged champagne bevels: ornament, not an invitation to act.
pub const DOCK_EDGE: Color = Color::srgb(0.48, 0.43, 0.33);
/// Warm ivory engraved into the dark furniture.
pub const DOCK_INK: Color = Color::srgb(0.94, 0.91, 0.83);
/// The five stationary inlays, in WUBRG order; never a rules signal.
pub const DOCK_INLAYS: [Color; 5] = [
    Color::srgb(0.72, 0.68, 0.52),
    Color::srgb(0.24, 0.45, 0.61),
    Color::srgb(0.43, 0.35, 0.50),
    Color::srgb(0.62, 0.32, 0.25),
    Color::srgb(0.29, 0.49, 0.39),
];
/// Recessed backing within a seat bar's measured ink bounds.
pub const SEAT_BACKING: Color = Color::srgba(0.055, 0.075, 0.090, 0.94);
/// The shallow channel carrying a whole phase, not an individual button.
pub const SEAT_TRACK: Color = Color::srgba(0.015, 0.024, 0.032, 0.62);
/// Small standing-stop accent, distinct from the current phase's gold fill.
pub const SEAT_STOP: Color = Color::srgb(0.88, 0.83, 0.69);
/// Pointer feedback on a phase tile; not a standing-order fill.
pub const SEAT_STEP_HOVER: Color = Color::srgba(0.88, 0.83, 0.69, 0.10);

/// Parchment, matching the middle of
/// [`tabletop::parchment`](baylee_client_core::tabletop::parchment) —
/// and, since this value, the card shader's own `PARCHMENT`.
///
/// Felt is the ground, parchment is a sheet, brass draws the lines and
/// gold belongs to the local seat. A panel that is *read* is a sheet; a
/// panel that is *worked in* stays [`PANEL`]. That is the whole rule, and
/// it is why the prompt is parchment and the seat tabs are not.
///
/// It was #EDE3CC while the saga page the card shader draws was #E0D4B0,
/// which is one material in two colours — and the wrong way round, the
/// *smaller* surface being the darker one where a colour field the size
/// of a fingernail already reads darker than a sheet of paper. The card's
/// is what both took. `the_parchment_is_the_same_paper_in_both_languages`
/// reads it back out of the WGSL, the way the rail and the plate are
/// already held.
pub const PARCHMENT: Color = Color::srgb(0.88, 0.83, 0.69);
/// Where a sheet lying on a table loses the light — its border, and the
/// rim of the generated sheet it has to meet.
pub const PARCHMENT_EDGE: Color = Color::srgb(0.669, 0.596, 0.433);
/// Ink.
pub const PARCHMENT_INK: Color = Color::srgb(0.098, 0.082, 0.062);
/// The quieter ink: hints, tallies, what a key does.
pub const PARCHMENT_SOFT: Color = Color::srgb(0.290, 0.251, 0.204);
/// Brass: the answer a sheet is asking for.
pub const BRASS: Color = Color::srgb(0.788, 0.635, 0.153);
/// The shadow a sheet lying above the table casts.
pub const SHEET_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.66);

/// The prompt slip's own ink, and the aside's.
///
/// [`PARCHMENT_INK`] and [`PARCHMENT_SOFT`] with a little of the sheet
/// showing through. The alpha is the whole point: the slip is written
/// *on* the parchment rather than printed over it, so the grain the sheet
/// carries comes up through the letters instead of stopping at them. It
/// is small — at 0.94 a stroke is still a stroke — because ink you can
/// see the table through is a watermark, not a question.
pub const SLIP_INK: Color = Color::srgba(0.098, 0.082, 0.062, 0.94);
/// The quieter of the two, for a hint or a tally.
pub const SLIP_SOFT: Color = Color::srgba(0.290, 0.251, 0.204, 0.92);
/// An aside — whatever the sheet said in brackets.
///
/// Grey rather than a lighter ink: an aside is a different *kind* of
/// sentence (a key to press, a count already implied by the board), and
/// draining the warmth out of it says so where another shade of brown
/// would only say "further away".
pub const SLIP_ASIDE: Color = Color::srgba(0.404, 0.376, 0.337, 0.86);
/// The colour a nib bleeds into the fibres around it.
///
/// Iron-gall ink does not cast a grey shadow; it wicks outwards and the
/// front of it is warm amber-brown, because the iron oxidises before the
/// gall darkens. That is the whole reason [`super::bleed`] is not
/// [`SLIP_SHADOW`] at a different alpha: the old shadow was the right
/// idea in the wrong colour and the wrong direction.
pub const BLEED: Color = Color::srgb(0.290, 0.200, 0.090);
/// The fill under an answer that is not the one the sheet is asking for.
///
/// Not [`Color::NONE`]: a button with no fill and a drop shadow renders
/// as a dark rounded hole in the parchment, because the shadow is drawn
/// under a surface that is not there. A few percent of the sheet's own
/// colour is enough to be a surface and not enough to compete with
/// [`BRASS`].
pub const SLIP_GHOST: Color = Color::srgb(0.832, 0.779, 0.639);

// ------------------------------------------------------ a dialog's own
//
// `docs/proposals/redesign-proposal.md` §1.3 draws the line and §6 applies it:
// **parchment is a sheet you read from, a panel is a place you work.**
// The prompt slip and the ability sheet are read; the zone browser has a
// checkbox, a tally and a Confirm in it, and that is work. So it is a
// panel — and a *warm* one, because the table it lies on is candlelit and
// [`PANEL`]'s cool near-black is the one surface in this client that was
// never on it.

/// The dialog's ground.
pub const DIALOG: Color = Color::srgb(0.110, 0.098, 0.075);
/// A dialog's ground where it is lifted: the head and the footer bands.
pub const DIALOG_LIT: Color = Color::srgb(0.149, 0.129, 0.098);
/// Every line drawn on a dialog: its border, and the rules between rows.
pub const DIALOG_LINE: Color = Color::srgb(0.216, 0.188, 0.122);
/// What a dialog says.
pub const DIALOG_INK: Color = Color::srgb(0.925, 0.890, 0.816);
/// The quieter half of it: a type line, a tally, a badge.
pub const DIALOG_SOFT: Color = Color::srgb(0.557, 0.514, 0.424);
/// The same quiet ink, on the **actions row**, which is no longer opaque.
///
/// [`DIALOG_SOFT`] reads 4.69 : 1 on an opaque [`DIALOG`] — 0.19 of
/// headroom over the 4.5 prose needs, which is all it ever had. The owner
/// asked for the row to be "a little bit transparent" on 14.09.2026
/// (`ax-design.md` §3.3 had said opaque, with a reason, and this reverses
/// it), and the row is composited over the sky: at a density of 0.92 the
/// same ink falls to 4.07 : 1 at the cloth's worst pose, and the sentence
/// standing on the row stops being readable.
///
/// So the ink pays for the transparency, where the transparency is:
/// `DIALOG_SOFT` scaled by 1.33 in **linear** light, which keeps its hue
/// exactly and takes it to 5.18 : 1 at rest and 4.95 : 1 at the worst
/// pose. `frontal.rs`'s
/// `the_rail_stays_readable_at_every_pose_the_cloth_can_take` is what
/// holds it, scanning the poses the two clocks can actually reach.
///
/// Only for ink standing **directly on the row**: the waiting sentence,
/// its bracketed asides, and the pool's own word. A keycap's legend sits
/// on the cap's own opaque fill and keeps [`DIALOG_SOFT`]; so does every
/// line in the drawer, which is opaque [`DIALOG`]; and the zone dialog —
/// which the owner named as the colour to match — is not touched at all.
pub const LEDGE_SOFT: Color = Color::srgb(0.635, 0.586, 0.485);
/// What a dialog says about a thing that is not there.
///
/// Quieter than [`DIALOG_SOFT`], and **deliberately** under the 4.5 : 1
/// that prose has to clear: 3.08 : 1 on [`DIALOG`], which is above the
/// 3.0 a large glyph or a disabled control is held to and below the ratio
/// that would make it read as something to attend to. AX §3.2 names the
/// number, `a_candle_is_dark_enough_to_write_on` holds both ends of it.
///
/// Two readers, and they are the same claim twice: the em dash standing
/// where an empty mana pool's entries would be, and — from §10.2 step 5 —
/// a draw offer the engine would refuse. Not [`DEAD`], which is the cool
/// near-black grey of the old panels and has no business on a candlelit
/// shelf.
pub const LEDGE_DEAD: Color = Color::srgb(0.430, 0.400, 0.330);
/// Candle: an offer, at the energy of something the engine is asking for.
///
/// The one hue the redesign leaves for "this is live" — [`ACCENT`]'s
/// teal is what it replaces. [`BRASS`] is its neighbour and not its
/// twin: brass is gilt, the colour of a thing already *taken* (an armed
/// deed, a place in an ordering), and candle is the invitation.
pub const CANDLE: Color = Color::srgb(0.878, 0.604, 0.227);
/// [`CANDLE`] laid over a dialog: the fill under a chosen row.
///
/// A wash rather than a fill, because a row that is chosen is still a row
/// being read — the tick and the name carry the claim, and a bar of
/// saturated candle across the list would make the chosen row the only
/// thing on the sheet anyone can see.
pub const CANDLE_WASH: Color = Color::srgba(0.878, 0.604, 0.227, 0.10);
/// The rail down the edge of the row the keyboard is standing on.
///
/// [`CANDLE`] dimmed, which is the grammar's own word for it: bright
/// candle is an offer the *engine* made, and this is the client
/// volunteering where the next press would land — a claim about the
/// keyboard, not about the game. A rail and not a wash because
/// [`CANDLE_WASH`] already means "chosen" on the same row, and the focus
/// stands on rows that are not chosen and leaves rows that are.
///
/// More opaque than either wash and still short of [`CANDLE`], because a
/// two-pixel edge and a full-width fill do not read alike at one alpha:
/// the wash is faint precisely because it covers the whole row.
pub const CANDLE_EDGE: Color = Color::srgba(0.878, 0.604, 0.227, 0.55);
/// The veil drawn over the table behind a dialog that holds the whole
/// answer, at full strength.
///
/// **Cold, and that is the whole of it.** A dialog this dark cannot be
/// separated from a veiled table by brightness — [`DIALOG`] is srgb8
/// (28, 25, 19) and the veiled baize measures (15, 29, 26), so the panel
/// is the *darker* of the two in green and the difference cannot be read
/// as depth. What separates them is **temperature**: a blue-black veil
/// pulls the cloth towards teal and drains the leather rail of its
/// warmth, and the dialog and its [`CANDLE`] are then the only warm
/// things in the window. A pure-black veil would have darkened everything
/// and separated nothing.
///
/// It used to be the same blue-black the hand zone's own ground was, one
/// step deeper, and the argument for that was that the zone is itself a
/// veil over the felt so a second hue would read as two materials. **The
/// zone is no longer a veil.** The owner asked for a container on
/// 14.09.2026 and `crate::frontal` is what draws it: one dye,
/// [`DIALOG`], at 0.88 falling to 0.92, with the table still showing
/// through and nothing cool about it. So this alpha is now on its own —
/// the veil is over the *table*, the container is the dialog's own
/// colour, and the two no longer have to match.
///
/// The alpha was **measured on screen, not reasoned about**: a
/// `BackgroundColor` composites in linear space, where an alpha buys far
/// less darkening than sRGB arithmetic predicts — `frontal::SKIRT`
/// carries the same warning and the numbers that earned it. Measured at
/// 1728×1052 against a live search, one screenshot either side of the
/// same `Confirm`: the felt goes (29, 53, 43) → (15, 29, 26), the rail
/// (21, 51, 40) → (12, 29, 25), a seat bar's ink 173 → 100 — a little
/// over half, everywhere, and everything still legible. 0.60 was the
/// first value and took the felt only to (18, 34, 29), which read as
/// weather rather than as a table that has been put down.
pub const TABLE_VEIL: Color = Color::srgba(0.020, 0.030, 0.055, 0.70);
/// [`CANDLE_WASH`] with the pointer on it.
///
/// Stated rather than derived, because `Feel`'s default hover shades a
/// colour towards white and keeps its alpha: a wash at a tenth lifted
/// that way is a slightly paler tenth, which is not an answer a player
/// can see. Twice the wash is.
pub const CANDLE_WASH_LIT: Color = Color::srgba(0.878, 0.604, 0.227, 0.20);
