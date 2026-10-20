//! The ability sheet: actions on smoked glass beside their permanent.
//!
//! It replaces a row of buttons in the prompt bar. Those buttons carried the
//! ability's *cost* as their whole label — `{2}, {T}` — because a button on a
//! bar has room for four words, so a player read what an ability charged and
//! never what it did, and the bar was at the bottom of the window while the
//! permanent was on the table.
//!
//! The glass panel beside the card names the
//! permanent's name, and a numbered row per thing it can do with the
//! ability's own printed sentence on it. The cost is under the key that arms
//! it, in a narrow column down the left — the two marks a player scans rather
//! than reads, with the whole of the rest of the row left to the sentence.
//! Its dark surface and gold accents match the table controls.
//!
//! # Two trees, because two things change at different rates
//!
//! [`sync_ability_sheet`] builds it when what it *says* changes and
//! [`place_ability_sheet`] moves it every frame, which is the split the seat
//! bars already use and for a sharper version of the same reason: a card
//! glides. Nothing on this table is positioned directly — `table::sync_scene`
//! writes a `Motion` target and `table::glide` moves the card there — so a
//! sheet anchored to where the card is *going* arrives before the card does,
//! and a sheet anchored to where it was drawn last frame follows it honestly.
//!
//! The placement reads the rig the camera was set from rather than the
//! camera's propagated `GlobalTransform`, for the reason in
//! [`crate::hud::seatbar`]: `bevy_ui` runs `UiSystems::Layout` before
//! `TransformSystems::Propagate`, so a `Node` written from the propagated
//! transform is a frame stale.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
pub(crate) mod granted;
mod spawn;
use baylee_client_core::Prompt;
use baylee_client_core::abilitysheet;
use baylee_client_core::card_face::TextBlock;
use spawn::spawn_sheet;

/// The narrowest the sheet is drawn.
///
/// The sheet is as wide as what is written on it, between this and
/// [`SHEET_MAX`]. One fixed width was doing two jobs that pull opposite ways:
/// wide enough that a long sentence does not come out as a column of two-word
/// lines, and narrow enough that a short one is not a keycap alone on a field
/// of paper. A single number picks the losing side of one of them.
///
/// The floor is what keeps a sheet of one-word rows (`+1`, `Tap for {G}`) a
/// sheet rather than a label, and it is what the footer — two halves pushed
/// apart, and the widest fixed thing here — is measured against.
const SHEET_MIN: f32 = 230.0;

/// The widest.
///
/// A printed sentence at 13 px reaches about sixty characters across this,
/// which is where a line stops being comfortable to read; past it the rows
/// wrap, which is what a *maximum* is for. It also keeps a sheet standing
/// beside a permanent in the middle lane clear of both window edges, which
/// the fixed width was chosen for and is the one job it was doing well.
const SHEET_MAX: f32 = 440.0;

/// The air between the card and the sheet's near edge.
///
/// It has to clear the lift a hovered or armed card rises by, which is why it
/// is not smaller: the sheet is anchored to the card's *resting* pose
/// ([`crate::table::CardRest`]) and stays put, so a card that rises under the
/// pointer rises **towards** the paper. Anchoring to the drawn pose instead
/// would let the two keep their distance and drag the sheet about, which is
/// the thing being fixed.
const SHEET_GAP: f32 = 18.0;

/// The nub's side, before it is turned.
///
/// A square turned forty-five degrees, the way a loyalty badge's point is
/// ([`crate::manaui`]): its far vertex reaches `NUB·√2/2` past the sheet's
/// edge and the other half is under the paper. Small — it is a tail, and a
/// tail that could be mistaken for a control would be one.
const NUB: f32 = 14.0;

/// How close to the window's edge the sheet may come.
const SHEET_MARGIN: f32 = 12.0;

/// The sheet's own margin, left and right.
///
/// **The sheet does not carry it — every child does.** The wash under a row is
/// what says "this one", so it has to reach the paper's edge; a row inset by
/// the sheet's padding reads as a second, narrower object lying on it. The
/// first version got there by giving the row `-SHEET_PAD_X` as a margin and
/// the same number back as padding, which works and depends on two numbers
/// cancelling. Paying the margin per child is the same column of ink with
/// nothing to cancel: the row is simply a child of a sheet with no horizontal
/// padding, so it is full width by construction rather than by arithmetic.
const SHEET_PAD_X: f32 = 12.0;

/// The same, above and below.
const SHEET_PAD_Y: f32 = 10.0;

/// How wide a mana bubble's pip is drawn.
///
/// Larger than the cost column's [`COST_MARK`], and for the reason the two
/// are different things: a cost is a mark set beside a sentence at reading
/// size, and a pip on a bubble is the *whole* control — five of them are all
/// there is on the paper. This is about the size a mana symbol is printed at
/// in a card's own cost line.
///
/// It shipped at 26 and the owner asked for "etwas kleiner". The floor under
/// that is not the mark but the **target**: the disc plus [`POUR_RING`] on
/// every side is what a finger lands on, and the close button in
/// [`spawn_head`] argues 30 px as the size for a card game played on a
/// tablet. 21 + 2×5 is 31, so this is as small as the pip goes without the
/// bubble becoming harder to hit than the sheet it stands in for.
const POUR_MARK: f32 = 21.0;

/// The ring of paper round a pip, which is the part the pointer lands on.
///
/// The pip itself is a disc and a disc is a poor target, so the button is the
/// disc plus this on every side. It is also where [`PICK_RING`] is drawn — a
/// mark painted round the disc alone would be a second outline on the symbol
/// rather than a ring round the control. It did not shrink with [`POUR_MARK`]
/// for exactly that reason: the ring is the target, and a smaller mark needs
/// it more.
const POUR_RING: f32 = 5.0;

/// The air between two pips.
const POUR_AIR: f32 = 4.0;

/// How much of [`palette::DIALOG_LINE`] the header of pips is washed in.
///
/// The hairline that parts the head from the list, widened into a field. The
/// owner asked for the mana row to stand out, and the sheet has exactly three
/// registers to say it in: brass says a *state* (picked, armed), ink says the
/// *pointer* (a hover, the close button's dish), and this says *structure* —
/// the border, [`rule`], the cost title's hairline. "This is not an item in
/// the list" is a structural claim, so it is drawn in the structural ink.
/// Brass here would be one very wide armed row; ink would be a row the
/// pointer never left.
///
/// Alpha rather than a mixed colour, so the grain comes through it as it comes
/// through every other wash on this paper. Measured over the sheet's own face
/// (203,187,148) it composites to 192,175,135 — 1.14:1, a *shade* where every
/// state on this sheet is a light, which is what keeps a header from ever
/// being mistaken for a row saying something.
///
/// Only the sheet gets it. On a bubble the pips are the whole paper, and a
/// band the size of the page is a differently-coloured page.
const POUR_BAND_WASH: f32 = 0.35;

/// The air over and under the pips inside that band.
///
/// So the 31-pixel targets have paper around them rather than sitting on the
/// band's own edges: 3 + 31 + 3 is a 37-pixel field.
const POUR_BAND_AIR: f32 = 3.0;

/// The paper between the head's hairline and the band.
///
/// With [`rule`]'s own 2 px under it that is six pixels, which is what keeps
/// the two from reading as one thick line. Under the band the margin stays
/// what it always was, because the band *parts* the head from the list and a
/// second hairline below it would be saying the same thing twice.
const POUR_BAND_GAP: f32 = 4.0;

/// How far a pip grows under the pointer, and gives way under a press.
///
/// Four times an ordinary button's [`crate::ambience::BUTTON_LIFT`], and that is the
/// same decision as the ring above read from the other end. 2.5% of a keycap
/// is half a pixel and is meant to be felt rather than seen, because a keycap
/// sits in a row of writing that must not move with it. A pip sits in air: it
/// is one mark, the paper it is on carries nothing else, and the owner asked
/// for an effect he could *see*. Ten per cent of 31 px is three pixels of
/// travel, which is a movement rather than a shimmer.
const POUR_LIFT: f32 = 0.10;

/// The legend on a row's keycap, which is what sets that cap's own side.
///
/// The footer's key is smaller, which is the whole reason [`KEYCAP_SIDE`] is
/// a ratio; this is the rows' half of that pair. Named because the column
/// under the cap has to measure itself against the cap.
const ROW_CAP_PT: f32 = 11.0;

/// The size a row's writing is set at.
///
/// Named because a row now sets its words and its marks at two sizes and the
/// pair has to be read together: this is the smaller of them, and it is what
/// a printed sentence is set in.
const ROW_PT: f32 = 13.0;

/// The size a row's **cost** is drawn at, disc for disc.
///
/// Larger than [`ROW_PT`], which is the one place on this sheet where a mark
/// is not sized off the sentence beside it — because under the keycap it is
/// not in a sentence any more. It is the answer to "what does this charge",
/// read on its own, and the number inside a loyalty badge is the smallest
/// thing in it: a badge is drawn at its size and its numeral at 0.82 of that,
/// so a cost set at the rows' own 13 pt put a planeswalker's loyalty on the
/// page at nine pixels. Sixteen puts it at thirteen — the size of the
/// sentence it is charging for, which is the right place for it to land.
const COST_MARK: f32 = 16.0;

/// The **widest** a cost column may be: three marks and the air between them.
///
/// A cap and not a width. The column is as wide as the widest cost on the
/// page and no wider, because paper between a cost and the sentence it
/// charges for is paper the sentence is not using — a planeswalker's badge is
/// one mark across and was being given a column for three.
///
/// Past three marks a cost stops reading as a column and starts reading as a
/// second sentence, so it wraps instead: `{W}{U}{B}{R}{G}` folds to three
/// marks over two rather than taking the room from the prose.
const COST_MARKS: f32 = 3.0 * COST_MARK + 2.0 * crate::manaui::air(ROW_PT);

/// The air over and under the hairline beneath a cost set as a title.
const COST_TITLE_AIR: f32 = 3.0;

/// How much of [`palette::DIALOG_LINE`] a title's hairline is drawn in.
///
/// Softer than [`rule`], which parts the sheet's *kinds of writing* — the
/// name from the rows, the rows from the footer. This one parts a cost from
/// the sentence it charges for, and those two are one thought: a hinge rather
/// than a wall, so it is the same ink at not quite half strength. Alpha and
/// not a mixed colour, so an armed row's brass comes through it exactly as it
/// comes through the paper.
const COST_RULE_WASH: f32 = 0.45;

/// The wash under the row that is armed — [`palette::BRASS`] at 16%.
///
/// It sits on an opaque sheet and not on felt, which is the whole reason a
/// translucent fill is allowed here at all: parchment over dark cloth goes
/// grey, and parchment over parchment is warmer parchment.
const ARMED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.16);

/// How much ink the pointer presses into a row — 10%.
///
/// **The hover is a different register from the state, and that is the whole
/// of it.** The washes above say what a row *is* — the keyboard is here, this
/// one is armed — and they say it in brass. The hover says where the pointer
/// is, and saying it with more brass is what made it nearly invisible: a
/// row's hot end was the next wash up the same ladder, which is eight per
/// cent of a warm hue on a warm ground, about eleven levels and in the blue
/// channel alone. A *picked* row had it worse than that — it rested at
/// [`PICKED_WASH`] and rose to [`PICKED_WASH`], so the row a player was
/// already pointing at answered the pointer with nothing at all.
///
/// Ink is the register this sheet already keeps for that, in
/// [`CLOSE_REST`]/[`CLOSE_HOT`]: a press in the paper, which is a fall in
/// luminance and reads on any ground and at any hue. So a row keeps its brass
/// and takes ink on top of it, and [`pressed`] is the one rule that says so
/// for all three states instead of three hand-tuned literals.
const ROW_PRESS: f32 = 0.10;

/// The dish the close button sits in — [`palette::INK`] at 5%.
///
/// A resting state at all, which the other controls on this sheet do without,
/// because a **finger** has no hover: on a tablet the cross would otherwise be
/// a glyph floating on paper with nothing to say it is a target. Five per cent
/// is a press in the paper rather than a button on it.
const CLOSE_REST: Color = Color::srgba(0.92, 0.77, 0.46, 0.08);

/// The same with the pointer on it.
const CLOSE_HOT: Color = Color::srgba(0.92, 0.77, 0.46, 0.20);

/// The wash under the row the keyboard is on.
///
/// It carries the cursor's **extent** — which row, edge to edge — and
/// [`PICK_MARK`] carries the claim. Two strengths of one wash is what the
/// owner could not see: the cursor and the arming are different claims about
/// the same row (*where a key would land* against *what a key has already
/// done*), and a row that is both showed only the louder of them.
const PICKED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.08);

/// The ink the keyboard's own cursor is drawn in — [`palette::CANDLE`].
///
/// **Brass at ink weight says where the keyboard is; brass as a light says
/// what it has done.** A mark two or three pixels wide is carried by its
/// luminance and [`palette::BRASS`] has none to give here: on the paper, on
/// the band and on [`ARMED_WASH`] it measures 1.28, 1.12 and 1.23:1, which is
/// a mark that is not there. The ink weight measures 3.73, 3.28 and 3.59:1 —
/// past WCAG 1.4.11's 3:1 for a graphical control — on all three, which is
/// what [`the_cursors_mark_carries_on_every_ground_it_lands_on`] holds.
///
/// It is carried as a `BorderColor`, which `Feel` never writes, so the state
/// and the hover stay in separate registers and a row can be armed *and* hold
/// the cursor at once. The cost is that this accent now says two things on
/// one sheet — whose list it is, in the head's letters, and where the keyboard
/// is, as a bar — told apart by shape.
const PICK_MARK: Color = palette::CANDLE;

/// How wide the bar down the left edge of the row the keyboard is on.
///
/// A marginal mark, on the edge the eye enters a row by, and not a ring: the
/// row is full-bleed against the sheet's own one-pixel border, so a ring
/// would double that line.
///
/// **Every row carries the border**, transparent at rest, and so does
/// [`spawn_pager`] — a border is box space in taffy, so a border on the picked
/// row alone would jog that one sentence three pixels right, and "one straight
/// left edge down the whole sheet" is the row's whole layout argument. That is
/// the price of an edge that never moves, and the reason this is a border
/// rather than a child spawned on the picked row.
const PICK_BAR: f32 = 3.0;

/// And the ring round the pip it is on.
///
/// A disc takes a ring, because there the ring *is* the control's extent —
/// which is why a pip needs no wash beside it and [`PICKED_WASH`] is a row's
/// alone. It eats into [`POUR_RING`] rather than adding to it, so the 31-pixel
/// target is exactly the size it was, and it lifts with the disc under
/// [`POUR_LIFT`] because `Feel` scales the node the border is on.
const PICK_RING: f32 = 2.0;

/// A row's resting wash with the pointer's ink pressed into it.
///
/// Source-over, so it is the colour the two layers actually make and not an
/// average of them: a row resting at nothing comes out as plain ink at
/// [`ROW_PRESS`] — the close button's register, exactly — and a row resting
/// at brass keeps the brass and darkens, which is the same movement either
/// way. `Feel` crossfades one colour into another and cannot stack two, which
/// is why this is composited here and not layered there.
fn pressed(rest: Color) -> Color {
    let rest = rest.to_srgba();
    let ink = palette::INK.to_srgba();
    // Never zero — the ink is opaque enough to divide by on its own — so the
    // resting alpha is free to be zero.
    let alpha = ROW_PRESS + rest.alpha * (1.0 - ROW_PRESS);
    let mix =
        |over: f32, under: f32| (over * ROW_PRESS + under * rest.alpha * (1.0 - ROW_PRESS)) / alpha;
    Color::srgba(
        mix(ink.red, rest.red),
        mix(ink.green, rest.green),
        mix(ink.blue, rest.blue),
        alpha,
    )
}

/// The parent of the sheet, so one despawn clears it.
#[derive(Component)]
pub struct AbilitySheetRoot;

/// The sheet, and where it was last put.
#[derive(Component)]
pub struct AbilitySheet {
    /// The permanent it belongs to.
    pub object: ObjectId,
    /// Where the two pieces already are, so a camera standing still costs
    /// one comparison instead of a relayout of the whole sheet.
    placed: Option<Placement>,
}

impl AbilitySheet {
    /// The rectangle the placed sheet occupies in logical pixels, once
    /// [`place_ability_sheet`] has laid it out.
    #[must_use]
    pub(crate) fn placed_rect(&self) -> Option<Rect> {
        self.placed
            .map(|at| Rect::from_corners(at.corner, at.corner + at.size))
    }
}

/// Where the sheet and its nub were last put.
///
/// The sheet's own corner is **not** enough to guard on any more. It is
/// clamped to the window, so a card gliding along the bottom row moves the
/// nub while leaving the corner exactly where it was — and a guard that only
/// watched the corner would pin the tail to where the card used to be. What
/// changed is that the sheet stopped being the only thing this system places.
#[derive(Clone, Copy, PartialEq)]
struct Placement {
    /// The sheet's top-left.
    corner: Vec2,
    /// The box the sheet came out at, which `bevy_ui` measured a frame ago.
    ///
    /// Carried rather than re-read because a *rebuilt* sheet has to be put
    /// back before it has been laid out — see [`put_sheet`].
    size: Vec2,
    /// The card's projected centre.
    mid: Vec2,
    /// The box the card covers.
    card: Vec2,
    /// Whether the sheet ended up under the card rather than over it.
    below: bool,
}

/// Puts the sheet at a placement.
///
/// A free function because two callers need it and only one of them is
/// [`place_ability_sheet`]: the other is the **spawner**, which is why the
/// sheet no longer blinks. The tree is rebuilt whenever a row is picked or
/// armed, and a rebuilt sheet used to be spawned hidden and revealed by the
/// placer a frame later — once per click, which is what the owner saw as a
/// flicker. A rebuild changes colours and not geometry, so the old
/// placement is still true and the new sheet is simply put back where the old
/// one stood; the placer then re-checks it against the card as it does every
/// other frame.
fn put_sheet(at: &Placement, node: &mut Node) {
    node.display = Display::Flex;
    node.left = px(at.corner.x);
    node.top = px(at.corner.y);
}

/// Puts the nub on the edge of the sheet that faces the card, at the card's
/// own centre — and clamped inside the sheet's straight run, because a point
/// growing out of a 6-pixel rounded corner reads as a chip out of the paper.
/// A sheet pushed against the window's edge by a card in the corner is
/// exactly where that happens.
fn put_nub(at: &Placement, node: &mut Node, edge: &mut BorderColor) {
    let reach = NUB * std::f32::consts::SQRT_2 / 2.0;
    let low = at.corner.x + 6.0 + reach;
    let high = at.corner.x + at.size.x - 6.0 - reach;
    node.display = Display::Flex;
    node.left = px(at.mid.x.clamp(low, high.max(low)) - NUB / 2.0);
    node.top = px(if at.below {
        at.corner.y - NUB / 2.0
    } else {
        at.corner.y + at.size.y - NUB / 2.0
    });
    // Which two edges face the card. `UiTransform` turns the square a quarter
    // turn clockwise, so the box's `right` and `bottom` become the pair
    // pointing down and its `top` and `left` the pair pointing up. The other
    // two lie on the paper and carry no ink — see [`SheetNub`].
    let ink = palette::DIALOG_LINE;
    *edge = if at.below {
        BorderColor {
            top: ink,
            left: ink,
            right: Color::NONE,
            bottom: Color::NONE,
        }
    } else {
        BorderColor {
            right: ink,
            bottom: ink,
            top: Color::NONE,
            left: Color::NONE,
        }
    };
}

/// The tenth row, which turns the page.
#[derive(Component)]
pub struct SheetPager;

/// The cross in the head, which is `Esc` for a hand that is not on the
/// keyboard.
///
/// The footer already says the way out, and says it in the one register a
/// pointer cannot use: a keycap is a *reminder*, not a control, and a player
/// on a tablet has nothing to press. This is the same door with a hit box on
/// it, in the place every window in the world puts one.
#[derive(Component)]
pub struct SheetClose;

/// The tail on the sheet's edge that points at the card it belongs to.
///
/// A square turned forty-five degrees, drawn **after** the sheet: its inner
/// half lies over the paper, where parchment on parchment is invisible and
/// the sheet's own edge underneath it is covered for exactly the width of the
/// tail. So the seam closes with no clip and no second mesh — the loyalty
/// badge's trick, run the other way round.
///
/// The other way round because it has to be in *front*. Drawn behind, the
/// sheet's `BoxShadow` fell across the half that sticks out and the tail came
/// out a muddy grey beside bright paper. Which costs something: the border
/// can no longer be on all four edges and be hidden by the body, so
/// [`place_ability_sheet`] lights the two that face outwards and leaves the
/// two lying on the paper clear.
///
/// It is the **only** thing the sheet attaches to the table, and that is a
/// decision rather than an omission. A `SheetHalo` used to ring the card as
/// well — [`palette::DIALOG_LINE`] at the sheet's own weight, so paper,
/// tail and ring would read as one gesture in one material. On the running
/// client it read as a gold frame round the card instead, and the owner asked
/// for it off. The tail already says which card the paper is about, from the
/// paper's own side; a second mark saying it from the card's side is a light
/// on a border that carries three rules claims and nothing else.
#[derive(Component)]
pub struct SheetNub;

/// The one piece that is attached to something: the tail.
///
/// `Without<AbilitySheet>` is not decoration — it is what lets a system hold
/// a mutable `Node` (or `UiTransform`) on the sheet and on this at once.
type Nub = (With<SheetNub>, Without<AbilitySheet>);

/// The movement the sheet is in the middle of.
///
/// Opening and closing are one component and not two states of the interface,
/// because the second one has to *outlive* the thing it is about: when
/// `Duel::ability_menu` goes back to `None` the sheet has nothing left to say
/// and is still on screen. So the closed branch of [`sync_ability_sheet`]
/// hands the tree over to [`zoom_the_sheet`] rather than despawning it, and
/// the despawn happens at the end of the movement.
#[derive(Component)]
pub struct SheetZoom {
    /// How far through it is: 0 at the first frame, 1 at the last.
    t: f32,
    /// Whether the end of it is a despawn.
    closing: bool,
}

impl SheetZoom {
    /// Whether this sheet is on its way off the screen.
    #[must_use]
    pub(crate) const fn is_closing(&self) -> bool {
        self.closing
    }
}

/// What the sheet is currently saying, so it is rebuilt only when that
/// changes.
///
/// The options themselves are fingerprinted by their labels rather than
/// stored: the list is rebuilt from `LegalActions` on every read anyway, and
/// what the sheet draws is exactly what those labels say.
#[derive(Resource, Default)]
pub struct SheetRevision {
    object: Option<ObjectId>,
    /// Which model the standing sheet is drawing.
    ///
    /// In the gate and not derived from `object`, because the same card can be
    /// both: a card in hand is cast through one chooser and may offer an
    /// activated ability through the other, and a sheet that changed model
    /// without changing card would otherwise keep the rows it had.
    source: Option<Source>,
    /// Whether the standing sheet was drawn with its cross.
    ///
    /// In the gate for the same reason `source` is: one card can be asked
    /// about by both owners in turn — this client's `CastMenu` puts the ways
    /// up, the row taken sends the deed, and the engine may ask its own
    /// `Prompt::CastMode` about the same card a frame later — and every other
    /// field in this gate can come out identical across that handover. Without
    /// it the cross stays on a question that can no longer be withdrawn.
    dismissible: bool,
    page: usize,
    pick: usize,
    armed: Option<usize>,
    fingerprint: Vec<String>,
    lang: Option<Lang>,
    /// How many printings had text when this was drawn.
    ///
    /// The same term `HudRevision` carries and for the same reason: what a
    /// row *says* is the card text, which arrives over the network after the
    /// sheet can already be opened. Nothing else in the fingerprint moves
    /// when it lands, so a sheet opened first would keep showing the fallback
    /// label for as long as it stood. The table's filing, not its size: a
    /// cached entry replaced by a fresher one changes words and not count.
    texts: u64,
}

/// Which of the two models a sheet is drawing.
///
/// The owner's answer of 14.09.2026: the cast-mode chooser and the ability
/// chooser are the **same piece of parchment beside the card**. §5 had put the
/// indexed chooser in the drawer and left this one where it was, and the
/// sentence §5 gave for keeping it — *it belongs to the card, not to the
/// question* — is just as true of the other one. So this panel has two models
/// and one renderer.
///
/// What it is **not** is a field on the button. A cast row is one row of an
/// indexed choice, and [`crate::hud::ChoiceButton`] already says exactly that:
/// `input::pick_choice` routes this client's own `CastMenu` first and the
/// engine's own `Prompt::CastMode` behind it, and `devctl` already reports
/// such a row under `choice`. A row that moved to this sheet keeps its click
/// path, its keys and its `/state` shape, and a reader asking for *abilities*
/// rightly does not see it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source {
    /// What a permanent can do. Rows can carry pips, and a row is armed
    /// before it is sent.
    Ability,
    /// The ways a card can be cast. There are never pips, nothing is armed,
    /// and a press answers.
    Cast,
    /// A temporary player action, identified independently of its departed source.
    Granted(baylee_core::ids::GrantedActionId),
}

/// One row of the sheet, whichever model opened it.
///
/// The renderer reads three things and no more, which is what lets two models
/// share one panel without a second copy of the paper: what the row costs,
/// what it says in the card's own words, and — only where the card says
/// nothing — the one line this client has for it.
struct SheetRow {
    /// Which of its model's options this row answers.
    ///
    /// Carried rather than taken from the row's position, for the reason
    /// [`crate::hud::ChoiceButton`] carries one: a list the player can filter
    /// draws a subset, and a button that answered by position would send the
    /// wrong one. It happens to equal the position for both models today —
    /// `Prompt::CastMode` numbers its options with the same `enumerate` the
    /// rows are built with — and the field is what keeps that a coincidence
    /// rather than a rule something later has to remember.
    answer: usize,
    /// The printed sentence, already split into rules text and reminder text.
    blocks: Option<Vec<TextBlock>>,
    /// What to write when there are no blocks, and **only** then.
    ///
    /// Not a second wording of a card, which is the thing the owner struck
    /// out (*„Bitte nicht custom texte für abilities verwenden, sondern die
    /// echten texte von skryfall"*). For an ability it is set only where
    /// `AbilityOption::printed_index` is `None` — the CR 305.6 mana of a basic
    /// land type, an ability granted by something this seat may not see, a
    /// prepared cast whose spell has no text to draw — none of which has a
    /// printed sentence this client can draw, so there is no first wording
    /// for it to be a second of. For a cast row it is
    /// `choices::cast_label`, which *is* the printed sentence where the card
    /// has one and the face's own name where the row is a face.
    line: Option<String>,
    /// What it costs, written so [`crate::manaui::spawn_rich`] can draw it:
    /// the printed head of [`Self::blocks`]' sentence, or the ability's own
    /// symbols where it has none.
    cost: Option<String>,
}

/// Which row of `options` the armed deed is, if any of them is.
///
/// By the action and not by an index, for the reason [`crate::Deed::Ability`]
/// carries the action: the list is rebuilt every frame and an index stored
/// across one of those rebuilds names whatever moved into that position.
fn armed_row(
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
) -> Option<usize> {
    let armed = duel.armed.as_ref()?;
    if armed.object != object {
        return None;
    }
    let action = match &armed.deed {
        crate::Deed::Ability(action) => action.clone(),
        crate::Deed::Run {
            then: crate::RunEnd::Ability(index),
            ..
        } => baylee_engine::choice::PlayerAction::ActivateAbility {
            source: object,
            ability_index: *index,
        },
        _ => return None,
    };
    options.iter().position(|o| o.action == action)
}

/// What a sheet is about, whichever model opened it.
///
/// One struct rather than a tuple, for the reason [`crate::hud::BrowserGate`]
/// is one: this is what the revision gate is compared against, and a field
/// left out of a tuple is a control that silently does nothing.
struct Opening {
    /// The card the sheet stands beside — a permanent on the table, or a card
    /// in the hand being cast.
    object: ObjectId,
    source: Source,
    /// The pip half, and empty on a cast sheet. Indexed the same as
    /// [`Self::rows`].
    options: Vec<crate::abilities::AbilityOption>,
    rows: Vec<SheetRow>,
    split: crate::abilities::Split,
    /// Which row the keyboard is standing on.
    picked: usize,
    /// Which row is armed, and never any on a cast sheet: a cast row answers
    /// rather than arming, which is what `abilitysheet::press` decides for an
    /// ability and what a mode has no equivalent of.
    armed: Option<usize>,
    /// Whether the sheet can be put down without answering it.
    ///
    /// True where a *menu of this client's* opened it, because the four doors
    /// out (the cross, `Esc`, a press outside, the row itself) all work by
    /// clearing that menu. False where the **engine** is the one asking: a
    /// `Prompt::CastMode` the engine put up is a question the table is waiting
    /// on, and nothing on this paper can withdraw it.
    dismissible: bool,
    /// One string per row, for the redraw gate.
    fingerprint: Vec<String>,
}

/// The sheet a permanent's abilities open.
fn ability_opening(duel: &Duel, lang: Lang, faces: &crate::cardtext::CardTexts) -> Option<Opening> {
    let object = duel.ability_menu?;
    let options = ability_options(duel, lang, object)?;
    if options.is_empty() {
        return None;
    }
    let view = duel.view.as_ref()?;
    let rows = options
        .iter()
        .enumerate()
        .map(|(at, option)| {
            let words = crate::abilities::printed_words(Some(faces), view, object, option);
            SheetRow {
                answer: at,
                // The printed head when the sentence gave one up; nothing
                // when it is drawn whole, since then it already says what it
                // costs; and the ability's symbols only where the card prints
                // no sentence at all.
                cost: match &words {
                    Some(cut) => cut.head.clone(),
                    None => option.cost.clone(),
                },
                // A prepared cast is the spell it casts: that card's name
                // over its whole text. A grant is its grantor's name over
                // the sentence that grants it.
                blocks: words
                    .map(|cut| cut.blocks)
                    .filter(|blocks| !blocks.is_empty())
                    .or_else(|| {
                        crate::abilities::prepared_words(faces, view, object, option)
                            .map(|(blocks, _)| blocks)
                    })
                    .or_else(|| {
                        crate::abilities::grant_words(faces, view, object, option)
                            .map(|(blocks, _)| blocks)
                    }),
                // Only where the card prints nothing for this row at all.
                // `printed_index` is `None` for exactly three things a
                // permanent can offer — the CR 305.6 mana of a basic land
                // type, an ability a continuous effect granted, a prepared
                // cast. The first is printed on no card, and this client's
                // one-line name is the only wording there has ever been. The
                // other two have blocks above wherever the view names their
                // card, and their line is that card's English name, or
                // "Granted ability" for a grantor this seat may not see.
                line: (option.printed_index().is_none() && !option.label.is_empty())
                    .then(|| option.label.clone()),
            }
        })
        .collect();
    Some(Opening {
        object,
        source: Source::Ability,
        split: crate::abilities::Split::of(&options),
        picked: duel.ability_pick,
        armed: armed_row(duel, object, &options),
        // `Duel::ability_menu` is this client's own and every door out clears
        // it.
        dismissible: true,
        fingerprint: options.iter().map(|o| o.label.clone()).collect(),
        rows,
        options,
    })
}

/// The sheet the ways of casting a card open.
///
/// **Two arrivals, one question, and both of them land here.** This client
/// builds a `Prompt::CastMode` of its own one step *before* the engine would
/// have built one — `CastMenu` stands while the engine is still holding an
/// ordinary priority window — and the engine asks the same question itself
/// whenever this client did not get there first. Reading only one of them
/// would draw one question beside the card on one route and in the drawer on
/// the other, which is a chooser that moves depending on how it arrived; the
/// drawer drops both for the same reason.
///
/// This client's own is asked first, for the reason `input::pick_choice` asks
/// it first: while it stands, it *is* the question, and the engine's priority
/// window behind it is not what the player is answering.
fn cast_opening(duel: &Duel, lang: Lang, faces: &crate::cardtext::CardTexts) -> Option<Opening> {
    // The engine's own is filtered on the turn the way the drawer filtered it,
    // because a question for somebody else is not this seat's to answer. This
    // client's own needs no such guard: nothing builds a `CastMenu` except the
    // press that opened it.
    let asked = || {
        let interaction = duel
            .interaction
            .as_ref()
            .filter(|_| duel.is_my_turn_to_act())?;
        let prompt = interaction.prompt();
        // The cursor with nothing on it stands on the first row, which is
        // where the other two sources start theirs. A sheet is a keyboard
        // surface — `1`..`9` answer it — so a sheet with no row lit would be
        // one that looks unreachable until an arrow key is pressed.
        casts_beside_card(&prompt).then(|| (prompt, interaction.chosen_index().unwrap_or(0), false))
    };
    let (prompt, picked, dismissible) = duel
        .cast_menu
        .as_ref()
        .map(|menu| (menu.prompt(), menu.pick, true))
        .or_else(asked)?;
    let Prompt::CastMode { object, .. } = &prompt else {
        return None;
    };
    let object = *object;
    let options = crate::choices::options(
        &prompt,
        lang,
        duel.statics.as_ref(),
        &duel.subtype_filter,
        crate::choices::FaceNames {
            view: duel.view.as_ref(),
            texts: Some(faces),
        },
    )?;
    if options.len() < 2 {
        return None;
    }
    let rows: Vec<SheetRow> = options
        .iter()
        .map(|option| SheetRow {
            answer: option.index,
            // A cast row has no printed *structure* to draw: `cast_label` has
            // already resolved it to one line, which for four of the six kinds
            // is the card's own sentence and for the other two is the face's
            // own name.
            blocks: None,
            line: (!option.label.is_empty()).then(|| option.label.clone()),
            cost: option.cost.map(|cost| cost.to_string()),
        })
        .collect();
    Some(Opening {
        object,
        source: Source::Cast,
        // No pips: a pip says what a *permanent* pours, and a card being cast
        // pours nothing.
        options: Vec::new(),
        split: crate::abilities::Split {
            pips: 0,
            rows: rows.len(),
        },
        picked,
        armed: None,
        dismissible,
        fingerprint: options.iter().map(|o| o.label.clone()).collect(),
        rows,
    })
}

/// Builds the sheet when what it says changes.
#[allow(clippy::too_many_arguments)]
pub fn sync_ability_sheet(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<SheetRevision>,
    existing: Query<Entity, With<AbilitySheetRoot>>,
    standing: Query<(Entity, &AbilitySheet)>,
    fonts: Res<UiFonts>,
    sheets: Res<UiSheets>,
    faces: Res<crate::cardtext::CardTexts>,
    settings: Res<crate::settings::ClientSettings>,
) {
    let lang = Lang::of(&settings.lang);
    // Cast first, for the reason `cast_opening` gives and `input::pick_choice`
    // already obeys. The two are never both open in practice — `activate_card`
    // clears one where it sets the other — and the order is written down
    // rather than relied on, because "they never stand together" is a claim
    // about two `Option`s that nothing enforces.
    let open = cast_opening(&duel, lang, &faces).or_else(|| ability_opening(&duel, lang, &faces));

    let Some(open) = open else {
        // Closed. Guarded on the revision so an interface with no sheet in it
        // does not run a query and a despawn loop on every frame of the game.
        //
        // The tree is *handed over* rather than despawned: what is on screen
        // has to shrink before it goes, and the thing it was about is already
        // gone. [`zoom_the_sheet`] owns it from here and does the despawn.
        if revision.object.is_some() {
            *revision = SheetRevision::default();
            for (entity, _) in &standing {
                commands.entity(entity).insert(SheetZoom {
                    t: 0.0,
                    closing: true,
                });
            }
        }
        return;
    };

    let object = open.object;
    // A cast sheet stands on its one page. `duel.ability_page` belongs to the
    // other model — `turn_the_page` writes it and `ability_menu_keys` turns it
    // with `0` — and a cast list has no counter of its own, so paging one
    // would draw a pager the keyboard could not turn. Nothing in this pool
    // reaches ten ways to cast a card; the day one does, this is where it asks
    // for a counter rather than quietly losing the tenth row, because the
    // pager is refused below for the same source.
    let page = match open.source {
        Source::Ability => abilitysheet::clamp(open.split.rows, duel.ability_page),
        Source::Cast | Source::Granted(_) => 0,
    };
    if revision.object == Some(object)
        && revision.source == Some(open.source)
        && revision.dismissible == open.dismissible
        && revision.page == page
        && revision.pick == open.picked
        && revision.armed == open.armed
        && revision.lang == Some(lang)
        && revision.fingerprint == open.fingerprint
        && revision.texts == faces.generation()
    {
        return;
    }
    // Read before the revision is overwritten: the sheet *opens* when it
    // starts being about a permanent it was not about a moment ago, and
    // merely redraws for every other reason it is rebuilt.
    let fresh = revision.object != Some(object);
    // Where the sheet that is about to be thrown away was standing. A rebuild
    // moves nothing — only the washes and the footer change — so the new one
    // is put straight back there and never has to be hidden for a frame while
    // it waits to be laid out. See [`put_sheet`].
    let standing = (!fresh)
        .then(|| {
            standing
                .iter()
                .find(|(_, sheet)| sheet.object == object)
                .and_then(|(_, sheet)| sheet.placed)
        })
        .flatten();
    revision.object = Some(object);
    revision.source = Some(open.source);
    revision.dismissible = open.dismissible;
    revision.page = page;
    revision.pick = open.picked;
    revision.armed = open.armed;
    revision.lang = Some(lang);
    revision.fingerprint.clone_from(&open.fingerprint);
    revision.texts = faces.generation();

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let root = commands
        .spawn((
            AbilitySheetRoot,
            crate::table::DuelStage,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            // The felt between the card and the sheet is not the sheet.
            Pickable::IGNORE,
            // This is a **root**, not a child of `HudRoot`, so the number
            // does not mean what the four in `hud::Z_LEDGE` means. bevy's
            // `ui_stack_system` sorts roots by `(GlobalZIndex, ZIndex)` and
            // then walks each subtree, so a root at `(0, 4)` stands over the
            // whole of a root at `(0, 0)` — this sheet is over every part of
            // the overlay, the prompt slip and the hover preview included,
            // which is *not* what it should be and is why the number is
            // written down here with what it actually does. Re-homing it
            // needs the third retained tree to be ordered against the first,
            // which is a decision of its own and not this one's to take.
            ZIndex(4),
        ))
        .id();
    let sheet = spawn_sheet(
        &mut commands,
        &fonts,
        &sheets,
        &faces,
        lang,
        &duel,
        object,
        open.source,
        &open.options,
        &open.rows,
        open.split,
        open.picked,
        page,
        open.armed,
        open.dismissible,
        fresh,
        standing,
    );
    let nub = spawn_trim(&mut commands, &sheets, fresh, standing);
    // Order *is* the drawing: the paper, then the tail lying across the
    // paper's edge. See [`SheetNub`] for why the tail is in front of the
    // sheet and not behind it.
    commands.entity(root).add_children(&[sheet, nub]);
}

/// The tail that points at the card the sheet is about.
///
/// It is spawned hidden unless the sheet it belongs to is being *rebuilt* and
/// `standing` says where the last one stood. [`place_ability_sheet`] is
/// otherwise what reveals it, and it needs a `ComputedNode` that does not
/// exist on the frame it is made — so without this it would be drawn once in
/// the window's top-left corner, which is where an unplaced absolute node is.
fn spawn_trim(
    commands: &mut Commands,
    _sheets: &UiSheets,
    fresh: bool,
    standing: Option<Placement>,
) -> Entity {
    let arrive = Vec2::splat(if fresh { 0.0 } else { 1.0 });
    let mut nub_node = Node {
        position_type: PositionType::Absolute,
        display: Display::None,
        width: px(NUB),
        height: px(NUB),
        border: UiRect::all(px(1)),
        ..default()
    };
    let mut nub_edge = BorderColor::all(Color::NONE);
    if let Some(at) = standing.as_ref() {
        put_nub(at, &mut nub_node, &mut nub_edge);
    }
    let nub = commands
        .spawn((
            SheetNub,
            nub_node,
            UiTransform {
                rotation: Rot2::radians(std::f32::consts::FRAC_PI_4),
                scale: arrive,
                ..UiTransform::IDENTITY
            },
            // The inset face leaves the two outward borders visible.
            BackgroundColor(Color::NONE),
            // Written by the placer, which is the only thing that knows which
            // two of the four edges are the ones facing out — or carried
            // straight over from the sheet this one replaces.
            nub_edge,
        ))
        .id();
    // Match the panel surface through the attached corner and its opening.
    let grain = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(nub).add_child(grain);
    nub
}

/// Follows the card with the sheet that is already built, and puts the nub
/// where it points at it.
///
/// Every frame, because the card is gliding — see the module header. The
/// write is guarded on where the two pieces already are, so a card standing
/// still costs one comparison and no relayout.
///
/// The anchor is the card's **resting** pose ([`crate::table::CardRest`]) and
/// not the pose it is drawn in. A card under the pointer rises 0.06 units and
/// grows 6%, and an armed one rises further still — so a sheet anchored to
/// what is drawn slid up the screen whenever the hand crossed the permanent
/// it was describing, and again the moment a row was armed. Neither is a
/// *move*: the card is exactly where it was.
///
/// **Two kinds of card, one placement.** The table's is a 3D pose that has to
/// be projected ([`crate::table::card_box`]); a card in the player's own hand
/// is `bevy_ui` and needs no projection at all, because the layout has already
/// put its node somewhere — in *physical* pixels, which
/// `ComputedNode::inverse_scale_factor` is the way back from. Both ends answer
/// the same pair, a centre and a box in logical pixels, and [`corner_for`]
/// takes it from there, so there is one rule about where a sheet goes rather
/// than two.
///
/// The hand half is not only 6c's groundwork. `abilities::options_for` reads
/// `LegalActions::abilities`, which names a source and an index and says
/// nothing about the zone it is in — so a card in hand offering two activated
/// abilities already opened this sheet, and the search below already failed to
/// find it. What the player saw was a sheet that never appeared.
///
/// [`crate::hud::HandRowCard`] and not the wider [`crate::hud::HandCardVisual`]:
/// the stack panel puts the latter on every row, and a spell on the stack is
/// not a card in the hand.
pub fn place_ability_sheet(
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    cards: Query<(&crate::table::CardVisual, &crate::table::CardRest)>,
    hand: Query<
        (
            &crate::hud::HandCardVisual,
            &bevy::ui::ComputedNode,
            &bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::HandRowCard>,
    >,
    mut sheet: Query<(
        &mut AbilitySheet,
        &mut Node,
        &mut Visibility,
        &bevy::ui::ComputedNode,
    )>,
    mut nub: Query<(&mut Node, &mut BorderColor), Nub>,
) {
    let Ok((mut sheet, mut node, mut seen, computed)) = sheet.single_mut() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let size = crate::hud::scale::space(window, ui.as_deref());
    // The camera is asked for *inside* the table's half and not before it: a
    // sheet standing beside a card in the hand is `bevy_ui` from end to end
    // and has no business waiting for a rig that has not settled yet.
    let on_the_table = shown.rig().and_then(|rig| {
        let lens = crate::table::Lens::new(rig, size);
        cards
            .iter()
            .find(|(visual, _)| visual.object == sheet.object)
            .and_then(|(_, at)| crate::table::card_box(&lens, &at.0))
    });
    // The table is asked first because that is where a permanent's abilities
    // are activated from, and an object is never in both lists at once.
    let Some((mid, card)) = on_the_table.or_else(|| {
        hand.iter()
            .find(|(visual, _, _)| visual.object == sheet.object)
            .map(|(_, computed, place)| {
                let scale = computed.inverse_scale_factor;
                (place.translation * scale, computed.size() * scale)
            })
    }) else {
        // The card is nowhere this system can point at — it left the table
        // and the hand, or the camera cannot see it. Hidden rather than
        // despawned, the way a seat bar is: the sheet is closed by the input
        // path and not by the camera.
        if node.display != Display::None {
            sheet.placed = None;
            node.display = Display::None;
            for (mut piece, _) in &mut nub {
                piece.display = Display::None;
            }
        }
        return;
    };
    // `ComputedNode` is bevy_ui's own layout, which is a frame old here for
    // the reason the module header gives. A frame is nothing to a sheet that
    // stands for as long as a player is reading it, and it is the only thing
    // that knows how tall — and, since the width became the text's to decide,
    // how wide — a sheet of text came out.
    let sheet_box = computed.size() * computed.inverse_scale_factor;
    if sheet_box.x <= 0.0 {
        // Not laid out yet. A sheet of size zero centres half a sheet to the
        // right of where it belongs, so it waits rather than being drawn
        // there for a frame — see the `Visibility::Hidden` it is spawned
        // with.
        return;
    }
    // The ceiling is the *window's* as well as the design's. `corner_for`
    // clamps a sheet that came out too wide back inside the left margin, but
    // it cannot make it narrower, so on a window under 404 logical pixels a
    // sheet at [`SHEET_MAX`] would hang off the right edge with rows on it.
    // Written unconditionally, ahead of the guard below: it is the input the
    // next frame's `sheet_box` is measured under.
    let ceiling = px((size.x - 2.0 * SHEET_MARGIN).clamp(SHEET_MIN, SHEET_MAX));
    if node.max_width != ceiling {
        node.max_width = ceiling;
    }
    let corner = corner_for(mid, card, sheet_box, size);
    let now = Placement {
        corner,
        size: sheet_box,
        mid,
        card,
        below: corner.y > mid.y,
    };
    if sheet.placed == Some(now) && node.display == Display::Flex {
        return;
    }
    sheet.placed = Some(now);
    put_sheet(&now, &mut node);
    // It has a place now, so it may be looked at.
    if *seen != Visibility::Inherited {
        *seen = Visibility::Inherited;
    }
    if let Ok((mut piece, mut edge)) = nub.single_mut() {
        put_nub(&now, &mut piece, &mut edge);
    }
}

/// Opens and closes the sheet, and despawns it at the end of a close.
///
/// The sheet grows, the nub follows it once it is nearly there
/// ([`motion::ZOOM_TAIL`]), and the same movement run backwards is what takes
/// them both away. A player who has turned motion off gets the end of it on
/// the first frame, which is the answer `table::glide` and `ShownRig` both
/// give.
///
/// The curve itself is [`super::motion`], because the drawer opens on it too
/// and §7 measures the whole shelf against *this* movement.
///
/// The despawn is here and not in [`sync_ability_sheet`] because a closing
/// sheet has outlived the thing it was about: `Duel::ability_menu` is already
/// `None` and the paper is still on screen.
pub fn zoom_the_sheet(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut sheets: Query<(&mut SheetZoom, &mut UiTransform, &ChildOf), With<AbilitySheet>>,
    mut trim: Query<&mut UiTransform, Nub>,
) {
    let still = prefs.all().reduce_motion;
    for (mut zoom, mut transform, parent) in &mut sheets {
        let span = if zoom.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        zoom.t = motion::step(zoom.t, span, time.delta_secs(), still);
        let done = zoom.t >= 1.0;
        if zoom.closing && done {
            // The whole tree, not the sheet: the nub is its sibling and the
            // root is what one despawn clears.
            commands.entity(parent.parent()).despawn();
            continue;
        }
        let scale = if zoom.closing {
            motion::shutting(zoom.t)
        } else {
            motion::opening(zoom.t)
        };
        transform.scale = Vec2::splat(scale);

        // The attached piece, on its own ramp — written every frame and
        // never guarded on `display`, because the placer is what reveals it
        // and a tail that was skipped while hidden would be shown at
        // whatever scale it was left at for the frame in between.
        let tail = motion::tail(zoom.t);
        for mut piece in &mut trim {
            piece.scale = Vec2::splat(if zoom.closing {
                1.0 - (1.0 - motion::ZOOM_FROM) * zoom.t
            } else {
                tail
            });
        }
    }
}

/// Where the sheet's top-left corner goes.
///
/// `mid` and `card` are the permanent's projected centre and the box it
/// covers ([`crate::table::card_box`]), `sheet` is how large the sheet came
/// out and `window` is the canvas.
///
/// Above the card by default, because that is where a sheet covers the fewest
/// other permanents: a board is read from the far edge towards the player's
/// own hand, so the space above a card holds what has already been read.
/// Below only when there is no room above — and *clamped* rather than allowed
/// to hang off either edge, because a sheet half outside the window is a list
/// with rows the player cannot see and cannot click.
///
/// A **mana bubble is placed by the same rule**, which it briefly was not: it
/// was put under the card, on the words "ein kleiner Dialog direkt unter der
/// Karte". Measured on the running client, "under" in the land lane is on top
/// of the seat's own phase rail — the bubble opened across `CBT | ATK | BLK |
/// DMG` for as long as the pick lasted — and the owner asked for it over the
/// card instead, "vergleichbar zu dem Abilities-Dialog". So there is one rule
/// again rather than two, and the bubble is where a player is already looking
/// for a sheet.
///
/// The width is measured rather than assumed, the way the height already was:
/// the sheet sizes itself to its own text between [`SHEET_MIN`] and
/// [`SHEET_MAX`], so a constant here would centre a narrow sheet as if it were
/// a wide one and put it off-centre by half the difference.
fn corner_for(mid: Vec2, card: Vec2, sheet: Vec2, window: Vec2) -> Vec2 {
    let above = mid.y - card.y / 2.0 - SHEET_GAP - sheet.y;
    let below = mid.y + card.y / 2.0 + SHEET_GAP;
    // Under the report button's corner at the top (#309), as every panel.
    let top = if above >= TOP_CLEAR {
        above
    } else {
        below.min(window.y - SHEET_MARGIN - sheet.y)
    };
    Vec2::new(
        (mid.x - sheet.x / 2.0).clamp(
            SHEET_MARGIN,
            (window.x - SHEET_MARGIN - sheet.x).max(SHEET_MARGIN),
        ),
        top.max(TOP_CLEAR),
    )
}

#[cfg(test)]
mod tests;

/// The two bounds are a range, and the footer fits inside the narrow end of
/// it — two caps at the footer's own size, the gap between its halves, the
/// gap inside each and the sheet's two margins.
///
/// Held here and not in a test because every term is a constant: a test would
/// be the compiler's own arithmetic run a second time, at a moment when it is
/// too late to matter. This fails the *build* that narrows the floor past
/// what the footer needs.
const _: () = {
    assert!(SHEET_MIN < SHEET_MAX);
    assert!(2.0 * SHEET_PAD_X + 2.0 * (KEYCAP_SIDE * 10.5) + 8.0 + 2.0 * 5.0 < SHEET_MIN);
    // Three marks fit the cap and a fourth does not, which is the whole of
    // what "three marks wide" means — a row of [`COST_MARK`] discs with
    // `manaui::air` between them, and `{W}{U}{B}{R}{G}` folding to three over
    // two because five of them do not fit. The first line is [`COST_MARKS`]'s
    // own definition read back; it is here so that redefining it has to keep
    // meaning three.
    assert!(3.0 * COST_MARK + 2.0 * crate::manaui::air(ROW_PT) <= COST_MARKS);
    assert!(4.0 * COST_MARK + 3.0 * crate::manaui::air(ROW_PT) > COST_MARKS);
};

/// The placer, run.
///
/// "Declared but never wired" is a bug this client has shipped before, and a
/// system that follows a *card* cannot be checked by reading it: what it has
/// to get right is the round trip from a `Transform` on the table, through
/// the rig the camera was just set from, to a `Node` on the overlay.
#[cfg(test)]
mod running;

/// Resolution choices have no hand/battlefield card to anchor a casting sheet to.
pub(super) fn casts_beside_card(prompt: &Prompt) -> bool {
    use baylee_engine::choice::CastModeKind;
    matches!(prompt, Prompt::CastMode { options, .. } if !options.iter().any(|o| matches!(o.kind,
        CastModeKind::MutatePosition(_) | CastModeKind::Dungeon(_) | CastModeKind::DungeonRoom { .. }
        | CastModeKind::Colors(_) | CastModeKind::Counter(_)
    )))
}
