//! The zone browser, drawn.
//!
//! Every zone a choice can reach that the table cannot show: the cards the
//! engine is *showing* this seat, the stack, and every graveyard, exile pile
//! and command zone at the table.
//!
//! It opens by itself for a choice that needs it ([`Browser::wanted`]) and by
//! hand from the table — a tap on the top card of a pile opens that pile —
//! and a click on one of its rows goes through exactly the same
//! `activate_card` a click on the table does.
//!
//! # Why it is a panel and not a sheet
//!
//! `docs/proposals/redesign-proposal.md` §1.3 draws the line: **parchment is a sheet
//! you read from, a panel is a place you work in.** This was parchment, and a
//! grid of ten card columns, on the argument that a graveyard is something a
//! player *reads*. It is not: §6 draws a checkbox, a tally and a Confirm, and
//! that is work. So it is a dark panel with a list in it — each row a
//! checkbox, a thumbnail, a name, the cost in pips, the type line and the
//! zone it is in — and the chosen row goes candle, not teal.
//!
//! # The list is the default, and the grid is a mode
//!
//! The grid used to be what the list *replaced*, and that was one argument
//! short. The half that was right: **choosing is reading.** A fetchland
//! offers the whole library, and the answer to "which of these ninety lands"
//! is in the type line and the cost, which are words — so the view a player
//! is dropped into grows *down*, where a hundred cards are, and says the
//! three things about a card that a search is actually read for. A grid
//! answers "show me more at once" by growing sideways, which buys nothing at
//! all for those three facts: they fit in one measure and everything past it
//! is blank.
//!
//! The half that was wrong: not every opening of this panel is a choice. A
//! player tapping their own graveyard to see what is in it is *browsing*, and
//! browsing a pile of cards is exactly what a grid is for — the owner asked
//! for it as *"wie auf einer Produktseite"* on 14.09.2026. So the sideways
//! axis is not refused, it is **chosen**: [`ViewMode`] is three shapes for
//! the same rows, the detailed list is the default because choosing is the
//! costlier half, and the grid is a click away for the half that is looking.
//!
//! What none of the three may do is ask for a different picture. Card art is
//! fetched at one size and one only; see [`TRAY_BIG_THUMB_W`] for the
//! arithmetic that bounds all three views to it.
//!
//! There was a strip of pile chips above the sheet doing the by-hand job,
//! drawing the local seat's graveyard, exile and command zone as counts. It is
//! gone: those three piles stand on the felt now with a real stack of cards on
//! them, and two drawings of one zone in two renderers is what the command
//! zone's own well already replaced once. The pile *is* the button.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use crate::ui_button::UiButton;
use baylee_client_core::browser::{BrowseRow, BrowseZone, Browser, Names, ViewMode, grid_across};
use spawn::spawn_tray;

/// How long the sheet takes to arrive.
///
/// It was the only number here when the sheet had no way out, and it stays
/// what it was: long enough to be a movement, over before a player has
/// finished looking down at it.
const REVEAL_IN: f32 = 0.20;

/// And to be put away.
///
/// Shorter than the way in, which is [`super::motion::ZOOM_OUT`]'s rule — an
/// answer arriving and an answer being dismissed are not the same event, and
/// a dismissal that takes as long as the answer reads as reluctance. It is
/// nonetheless half as long again as that constant, because this sheet does
/// not shrink where it stands: it **travels**, the width of the window, and a
/// journey that is over in a tenth of a second is a jump.
const REVEAL_OUT: f32 = 0.16;

/// How small the sheet is by the time it reaches the tray.
///
/// Not the button's own size. A 1020-pixel sheet folded into a 24-pixel
/// button is a ratio of 0.024, at which the last third of the movement is a dot
/// sliding along the shelf — the *sheet* has already stopped being legible as
/// one. This is the size at which it still reads as the panel that was there
/// a moment ago, arriving at the button rather than replaced by it.
const REVEAL_FOLD: f32 = 0.08;

/// How far below its place the sheet starts, and rises through.
///
/// **Below**, not above: `bevy_ui`'s y runs downward, so the entrance writes
/// a *positive* offset that shrinks to nothing and the sheet comes up into
/// its place. A page being laid down would fall, which is the wrong reading
/// of a dialog that is being handed to the player rather than dropped in
/// front of them.
///
/// Neither way out has a use for it. Being put away is a journey and the
/// journey is the whole of its direction; being answered is a dismissal in
/// place, which is the one movement here that goes nowhere at all.
const REVEAL_RISE: f32 = 12.0;

/// Where the sheet is in its own opening, and which way it is going.
///
/// A resource and not a component on the panel, which is where [`DrawerZoom`]
/// and [`SheetZoom`] keep the same two fields. The difference is how often
/// the thing underneath is rebuilt: this panel is despawned and written again
/// on every keystroke in its filter box, and progress kept on it would restart
/// the entrance on each one. The drawer solves that by refilling a panel it
/// keeps ([`super::ledge::drawer::sync_drawer`]'s "an open panel is kept and
/// refilled"); this sheet is a hundred rows deep and is rebuilt whole, so the
/// state is kept beside it instead.
///
/// `t` is reset rather than reversed when the direction changes, for
/// `DrawerZoom`'s reason: the two spans are different and so are the two
/// curves.
///
/// [`DrawerZoom`]: super::ledge::drawer::DrawerZoom
/// [`SheetZoom`]: super::sheet::SheetZoom
#[derive(Resource, Default)]
pub struct TrayReveal {
    /// 0 at the start of the movement, 1 at its end.
    t: f32,
    /// Whether this is the way out — the movement at the end of which this
    /// system takes the sheet off the tree.
    closing: bool,
    /// Whether the way out is the **tray**, recorded while the sheet is up.
    ///
    /// A sheet the player opened is *put away*, and where it goes is the
    /// button that brings it back. A sheet a **question** opened is
    /// *answered*: it has no button — the tray's is held for exactly as long
    /// as the question owns the sheet — so a flight to it would be a
    /// movement towards a door the player was not allowed through, and
    /// `hud::motion`'s own rule says a dismissal is not a journey.
    ///
    /// Recorded when the sheet is built and not when it closes, because by
    /// then the browser is shut and a shut browser answers `for_choice` with
    /// false. It is the sheet's provenance, and the sheet is what is moving.
    to_tray: bool,
}

impl TrayReveal {
    /// Sends a standing sheet on its way out.
    ///
    /// Takes `showing` rather than reading the sheet itself, because the one
    /// thing this must not do is start a second flight on a sheet that is
    /// already in the air: the browser is shut for every frame of the way
    /// out, so a `closing` written unconditionally would hold `t` at nought
    /// and the sheet would hang over the tray for ever.
    fn send_away(&mut self, showing: bool) {
        if showing {
            self.closing = true;
            self.t = 0.0;
        }
    }

    /// Starts, or keeps, the entrance of a sheet that is being built.
    ///
    /// A sheet that was on its way out cannot be caught and turned round: its
    /// `t` runs on the closing span and against the closing curve. So it goes,
    /// and a fresh one opens in its place on the same frame — from the
    /// beginning, because what the player asked for is the sheet arriving and
    /// not the last two frames of it. A rebuild of a sheet that is simply
    /// *standing* keeps its `t`, which is what stops every keystroke in the
    /// filter box replaying the entrance.
    fn arrives(&mut self, fresh: bool, to_tray: bool) {
        if self.closing || fresh {
            self.t = 0.0;
        }
        self.closing = false;
        self.to_tray = to_tray;
    }
}

/// The sheet's entrance, its flight into the tray, and the despawn at the end
/// of one.
///
/// After [`sync_tray`] deliberately. `sync_tray` is what decides that the
/// sheet is leaving — it writes [`TrayReveal::closing`] instead of despawning
/// — and if this ran first, a sheet reopened on the frame its `t` reached 1
/// would be despawned here and rebuilt there, both of them acting on the same
/// entity in one frame. The despawn is here and not there for
/// [`super::ledge::drawer::zoom_the_drawer`]'s reason: a sheet on its way out
/// has outlived the question it was about, and `sync_tray` has already
/// written the reading that says so.
///
/// The flight is one interpolation of two things at once. The panel's centre
/// travels to [`super::ledge::tray::zones_button_centre`] and its scale falls
/// to [`REVEAL_FOLD`] — a `UiTransform` scales about the node's own centre, so
/// moving that centre onto the button's is the whole of "it went in there".
/// Both are read off the panel's own `Node`, which is where a drag has just
/// written the sheet's position; nothing here asks a `ComputedNode`, so there
/// is no logical-versus-physical question to get wrong.
#[allow(clippy::too_many_arguments)] // a Bevy system
pub fn reveal_tray(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    mut reveal: ResMut<TrayReveal>,
    mut panels: Query<(&Node, &mut UiTransform), With<TrayPanel>>,
    // The same two queries [`sync_tray`] tears the sheet down with, and
    // deliberately the same handle: what this despawns at the end of a
    // flight is exactly what that despawns on a rebuild, and two signatures
    // naming the same pair of nodes differently is how one of them comes to
    // be missing a node. `Or<(With<TrayBand>, With<TrayVeil>)>` says it in
    // one query and is `clippy::type_complexity`; two bare queries say it in
    // two and are an eighth argument. This says it in none of its own.
    tree: TrayTree,
) {
    if panels.is_empty() {
        return;
    }
    let still = prefs.all().reduce_motion;
    let span = if reveal.closing {
        REVEAL_OUT
    } else {
        REVEAL_IN
    };
    reveal.t = super::motion::step(reveal.t, span, time.delta_secs(), still);
    if reveal.closing && reveal.t >= 1.0 {
        for entity in tree.panel.iter().chain(tree.veil.iter()) {
            commands.entity(entity).despawn();
        }
        return;
    }

    let band = band_of(&windows, ui.as_deref());
    let target = super::ledge::tray::zones_button_centre(band);
    // Ease-out on the way in, ease-*in* on the way out: a thing arriving
    // slows into place and a thing leaving gathers speed, which is the same
    // pair `motion::opening` and `motion::shutting` are.
    let eased = if reveal.closing {
        reveal.t * reveal.t
    } else {
        1.0 - (1.0 - reveal.t).powi(3)
    };
    for (node, mut transform) in &mut panels {
        let (scale, travel, drop) = match (reveal.closing, reveal.to_tray) {
            // Put away: it folds down to [`REVEAL_FOLD`] and its middle
            // travels onto the tray button's.
            (true, true) => {
                let here = centre_of(node, band);
                (
                    1.0 - (1.0 - REVEAL_FOLD) * eased,
                    (target - here) * eased,
                    0.0,
                )
            }
            // Answered: it goes where it stands. `motion::shutting`'s range
            // exactly, because this *is* a dismissal and the drawer's is the
            // movement it should read like.
            (true, false) => (super::motion::shutting(reveal.t), Vec2::ZERO, 0.0),
            // Arriving, either way. One entrance, because a sheet is a sheet
            // however it was asked for, and a player who saw two would be
            // learning a distinction the interface does not otherwise make.
            (false, _) => (
                0.965 + 0.035 * eased,
                Vec2::ZERO,
                (1.0 - eased) * REVEAL_RISE,
            ),
        };
        transform.scale = Vec2::splat(scale);
        transform.translation = Val2::new(px(travel.x), px(travel.y + drop));
    }
}

/// The middle of the sheet, in the same coordinates the tray's button is
/// measured in.
///
/// The `Node`'s four numbers are `Val::Px` because `Placement` is written
/// there in pixels; anything else means the sheet has not been placed yet,
/// and falling back to the middle of the band is a flight that starts from
/// where a sheet with no placement would be drawn.
fn centre_of(node: &Node, band: (f32, f32)) -> Vec2 {
    let px_of = |v: Val, or: f32| if let Val::Px(n) = v { n } else { or };
    let width = px_of(node.width, Placement::DEFAULT_W);
    let height = px_of(node.height, Placement::DEFAULT_H);
    Vec2::new(
        px_of(node.left, (band.0 - width) / 2.0) + width / 2.0,
        px_of(node.top, (band.1 - height) / 2.0) + height / 2.0,
    )
}

/// The thumbnail on a row.
///
/// Small on purpose: it is there to be *recognised*, not read — the name is
/// beside it in full and the preview is a hover away. It is what sets the row
/// height, being the tallest thing in one.
///
/// It was 30, and the owner asked for a little more on 14.09.2026 — *"amche
/// die Zeilen etwas höher, damit man das Miniatur-Bild etwas größer sieht"* —
/// which is one number and four consequences, because a row is the unit the
/// whole sheet is measured in. The row grows 55.9 → 69.9, [`TRAY_ROWS`] drops
/// from 8.5 to 7.5 so the sheet opens 49 px taller instead of 119, and
/// `Placement::DEFAULT_W`/`MIN_W` take the ten pixels the column itself
/// gained. The three tests below the fold hold all of that together, so this
/// constant cannot be moved on its own.
const TRAY_THUMB_W: f32 = 40.0;
/// Its height, keeping the 63:88 card aspect.
const TRAY_THUMB_H: f32 = TRAY_THUMB_W * 88.0 / 63.0;
/// The air above and below the thumbnail in a row.
const TRAY_ROW_PAD: f32 = 7.0;
/// One row, which is the unit the whole sheet is measured in.
const TRAY_ROW_H: f32 = TRAY_THUMB_H + 2.0 * TRAY_ROW_PAD;
/// The picture in the large list, and in a grid tile at its smallest.
///
/// **73 and not a round number**, and it is the one measurement in this file
/// that is not a taste: card art is fetched at [`ArtSize::Small`], which is
/// 146×204 *physical* pixels, so on this retina screen a 73-wide picture is
/// exactly one texel to one pixel. Past about 100 the softening is visible,
/// and the next size up costs eleven times the texture — a hundred-card
/// library at [`ArtSize::Normal`] is 133 MB against a budget of 96 on a
/// phone. So the two views that make the picture bigger stop where the
/// picture does, and neither of them asks for a different image: a row and a
/// tile are the same [`ImageKey`] drawn at three sizes.
///
/// [`ArtSize::Small`]: baylee_client_core::images::ArtSize::Small
/// [`ArtSize::Normal`]: baylee_client_core::images::ArtSize::Normal
/// [`ImageKey`]: baylee_client_core::images::ImageKey
const TRAY_BIG_THUMB_W: f32 = 73.0;
/// Its height, the same 63:88 as every other card in this client.
const TRAY_BIG_THUMB_H: f32 = TRAY_BIG_THUMB_W * 88.0 / 63.0;
/// The air above and below the picture in the large list.
const TRAY_BIG_ROW_PAD: f32 = 8.0;
/// One row of the large list.
const TRAY_BIG_ROW_H: f32 = TRAY_BIG_THUMB_H + 2.0 * TRAY_BIG_ROW_PAD;
/// The checkbox in the large list, which grows with the row.
///
/// The one column that must not get *relatively* smaller as the row grows:
/// the large list is a list a player chooses from, and a tick target that
/// shrank as everything around it doubled would be the control going the
/// wrong way.
const TRAY_BIG_BOX: f32 = 18.0;
/// The widest a grid tile is allowed to grow.
///
/// The tiles share the width the way `seatbar`'s split rail shares the mat:
/// they grow together to this cap and the slack past it goes into the gaps,
/// never into the picture — so a tile is sharp at every width the sheet can
/// be dragged to, instead of being sharp at one of them.
const TRAY_TILE_MAX: f32 = 100.0;
/// The air between two tiles, both ways.
const TRAY_TILE_GAP: f32 = 10.0;
/// What a tile is wider than its picture: the focus rail on both sides, and
/// the two pixels of air that keep the picture off it.
///
/// It exists because the packing and the drawing have to agree about *which*
/// width they are sharing out. [`grid_across`] divides the measure among
/// whole tiles, so it has to be handed the whole tile — hand it the
/// picture's width instead and every full row is eight pixels per tile too
/// wide and wraps its last one onto a line of its own. Invisible with two
/// cards in a zone and certain on a graveyard, which is why it is a named
/// constant read by both halves rather than a `+ 4.0` written out at the one
/// that draws.
const TRAY_TILE_CHROME: f32 = 2.0 * (TRAY_FOCUS + 2.0);
/// The gutter every band of the sheet keeps at its left and right.
///
/// The rows carry it themselves rather than the panel carrying it for them,
/// which is what lets a chosen row's wash run from edge to edge: a highlight
/// that stopped short of the border would read as a chip lying on the list
/// rather than as the row being chosen.
const TRAY_SIDE: f32 = 16.0;
/// The air between two things in a row, and between two controls.
const TRAY_GAP: f32 = 11.0;
/// The checkbox. Also the width of the gutter a row that cannot be chosen
/// leaves empty, so the names stay in one column.
const TRAY_BOX: f32 = 15.0;
/// The rail down the left edge of the row the keyboard stands on.
///
/// Every row reserves it and only the focused one paints it, so the focus
/// moving never moves a name. Two pixels rather than one: the row already
/// ends in a one-pixel rule and a focus the same weight as a separator is a
/// separator.
const TRAY_FOCUS: f32 = 2.0;
/// One mana pip on a row.
const TRAY_PIP: f32 = 15.0;
/// What a cost is given: four pips and the air between them.
///
/// Four rather than the longest cost in the pool, because this is the width
/// the *sheet* is derived from and a cost longer than four pips simply pushes
/// the name's measure in. Most printed costs are three or four symbols.
const TRAY_COST_W: f32 = 4.0 * TRAY_PIP + 3.0 * 2.0;
/// What one character of the row's prose is worth, as a fraction of its size.
///
/// The same estimate `hud::stack` budgets a stack entry's name with. It is an
/// estimate and it is allowed to be: what it sizes is a *measure*, and a name
/// a little longer than one simply takes a little of the slack beside it.
#[cfg(test)]
const TRAY_CH: f32 = 0.52;
/// The name's size.
const TRAY_NAME_SIZE: f32 = 12.5;
/// The type line's.
const TRAY_TYPE_SIZE: f32 = 10.5;
/// The zone badge's.
const TRAY_BADGE_SIZE: f32 = 9.0;
/// The zone badge: the longest zone word this client has — `Kommandozone`,
/// which sets at 70.0 px in Alegreya Sans Medium at [`TRAY_BADGE_SIZE`]
/// times [`super::UI_SCALE`] — plus its padding and border.
///
/// **Measured in the shipped face, not estimated.** [`TRAY_CH`] is a mean
/// over mixed-case English prose and holds there to within a percent; a
/// German compound of round wide letters runs 0.64 per character, and the
/// estimate cut the last three letters off every badge on the panel. It was
/// 68.8 while the face was Inter, and re-measuring on the change of face is
/// what this doc is for: a number a *font* produced has to be taken from
/// the font that is shipped, and 1.2 px of it is one clipped letter.
const TRAY_BADGE_W: f32 = 70.0 + 12.0;
/// Thirty characters of name — `Sea Gate Loremaster` and room to spare.
#[cfg(test)]
const TRAY_NAME_W: f32 = 30.0 * TRAY_CH * TRAY_NAME_SIZE;
/// A type line's measure, at [`TRAY_TYPE_SIZE`] times [`super::UI_SCALE`] in
/// Alegreya Sans Medium.
///
/// It was 185.1 — `Legendary Planeswalker — Aminatou`, one line measured out
/// of one card — and one card is not a measure. Against the type lines the
/// **catalog** actually prints for this pool, 185.1 clips 14 rows of 1321;
/// against the two decks at the table it was first seen on, it clipped
/// **13 of 167**, because a Commander deck is made of legends and a legend's
/// type line is the long kind: `Legendary Creature — Human Warrior Ally` sets
/// 211.2. 215.0 leaves exactly one row of those 167 over the edge
/// (`Legendary Creature — Phyrexian Human Wizard`, 239.7) and costs the panel
/// 30 px of default width, which [`baylee_client_core::browser::Placement`]
/// carries.
///
/// A *measure* rather than a fit: a longer one is clipped at its end, not
/// wrapped — see [`clipped`] for why that sentence was false for as long as
/// this constant has existed. Measured rather than estimated for the reason
/// [`TRAY_BADGE_W`] gives, and measured in **German**, because the estimate
/// and the English line agreed with each other and both disagreed with the
/// screen.
const TRAY_TYPE_W: f32 = 215.0;

/// The panel's default width: **one row**.
///
/// Its fixed furniture — the checkbox, the thumbnail, four pips of cost and
/// the zone badge — plus a measure for each of the two pieces of prose, and
/// the gutters and gaps that hold them apart. Anything longer than a measure
/// takes the slack in the middle, which is where a list wants it.
///
/// The sheet takes its width from [`Placement`] now, because a player can
/// resize it. This stays as the *derivation* of that default — the arithmetic
/// that says why the number is what it is — and
/// `the_default_width_is_one_whole_row` holds the two together.
#[cfg(test)]
const TRAY_PANEL_W: f32 = 2.0 * TRAY_SIDE
    + TRAY_BOX
    + TRAY_THUMB_W
    + TRAY_NAME_W
    + TRAY_COST_W
    + TRAY_TYPE_W
    + TRAY_BADGE_W
    + 5.0 * TRAY_GAP
    + 2.0;

/// The title row: the sheet's name and the way out of it.
const TRAY_TITLE_H: f32 = 24.0;
/// One zone tab.
const TRAY_TAB_H: f32 = 22.0;
/// The search field, the sort key and the arrow beside it.
const TRAY_CTRL_H: f32 = 28.0;
/// A footer button.
const TRAY_FOOT_H: f32 = 34.0;
/// The air between the head's three rows.
const TRAY_HEAD_GAP: f32 = 8.0;

/// A window button in the head: square, so its mark has a centre to sit over.
///
/// Two pixels inside [`TRAY_TITLE_H`], which is what keeps the pair from
/// touching the rule under the row while still being the tallest thing in it.
const HEAD_BTN: f32 = TRAY_TITLE_H - 2.0;

/// The gap between the two of them.
///
/// Tighter than [`TRAY_GAP`]: they are one control in two halves — both
/// change the sheet's size — and a pair set at the row's own spacing reads as
/// two unrelated buttons that happen to be adjacent.
const HEAD_BTN_GAP: f32 = 4.0;

/// The mark in one, at the size the rest of the sheet's icons are set.
const HEAD_BTN_PT: f32 = 12.0;
/// The head band's own padding, above and below.
const TRAY_HEAD_PAD: f32 = 12.0;
/// The box at the start of a zone tab's name.
///
/// Smaller than the list's [`TRAY_BOX`], because it stands in a 22-px chip
/// rather than beside a 56-px thumbnail, and because a box as big as the one
/// that answers the question would be claiming to be that box.
const TRAY_TAB_BOX: f32 = 12.0;
/// The head band's own top corners: the panel's, less the border it sits in.
///
/// Derived and not chosen, so that moving [`super::SHEET_R`] moves both curves
/// together. `the_head_is_cut_concentrically_with_the_panel` is what holds it.
const TRAY_RADIUS: f32 = 5.0;
const TRAY_HEAD_R: f32 = TRAY_RADIUS - 1.0;
/// The footer band's.
const TRAY_FOOT_PAD: f32 = 11.0;

/// What stands above and below the list.
///
/// Arithmetic rather than a measurement, unlike the grid's chrome that came
/// before it: every row of the head and the footer is given an explicit
/// height here, so there is no text line box left to guess at. It exists so
/// [`Placement::DEFAULT_H`](baylee_client_core::browser::Placement::DEFAULT_H)
/// is a number with a reason rather than one somebody liked.
#[cfg(test)]
const TRAY_CHROME_H: f32 =
    // the head: its border, its padding, and two rows with air between them —
    // the tabs are in the title row now, so `TRAY_TAB_H` is spent inside
    // `TRAY_TITLE_H` rather than beside it
    1.0 + 2.0 * TRAY_HEAD_PAD + TRAY_TITLE_H + TRAY_CTRL_H + TRAY_HEAD_GAP
    // the footer: its border, its padding, one button
    + 1.0 + 2.0 * TRAY_FOOT_PAD + TRAY_FOOT_H
    // and the panel's own border, top and bottom
    + 2.0;

/// How many rows the sheet opens showing.
///
/// The half is the point: a row cut through by the bottom edge is what says
/// the list continues, and it says it without a scrollbar. A grid was cut to
/// four *whole* rows for the opposite reason — most of a fifth row of cards
/// was space nothing could ever be put in.
///
/// Eight and a half, and it has been eight and a half throughout. The rows
/// grew on 14.09.2026 (see [`TRAY_THUMB_W`]) and the count came down to seven
/// and a half with them — and then the zone tabs moved into the title row
/// later the same day, handed 30 px of chrome back, and the sheet spent them
/// on the row it had just given up. This comment said "seven and a half" for
/// four days with `8.5` written directly underneath it, and so did
/// [`Placement::DEFAULT_H`]'s. Neither could be caught by a test, because
/// `the_default_height_shows_half_a_row` reads the **constant** and the
/// constant was right the whole time.
#[cfg(test)]
const TRAY_ROWS: f32 = 8.5;

#[derive(Component)]
pub(crate) struct TrayScroll;

/// What the dialog was last drawn from.
///
/// The **sixth** retained tree in this client and the fifth revision counter
/// beside [`super::HudRevision`], and it exists for the reason every one of
/// the others does: `HudRevision` counts `hovered`, so it is rebuilt on every
/// pointer move that changes which object is under the cursor — and the
/// dialog draws a hundred rows that the pointer is moving *across*.
///
/// The owner reported it as instability: *„Das Zonen-Dialog ist noch sehr
/// instabil! Beim Hover flackert alles"*. The flicker is what a despawn and
/// respawn looks like from outside. Two halves of it were paid off in
/// 429e5a1a — the row no longer grows under the pointer, so a list stopped
/// reflowing while it was being read — and this is the third: a row torn down
/// and written again comes back with a fresh [`Feel`] at `warmth: 0`, and
/// picking needs a frame to send `Over` to the new entity, so the row under
/// the pointer goes dark for a frame *every time the pointer moves onto it*.
///
/// What is **not** here is as load-bearing as what is. The dialog reads no
/// hover at all: `Feel` lights a row through picking, frame by frame, without
/// anything being rebuilt, and `RowStanding::focused` is
/// `Interaction::aim` — the keyboard's row, not the pointer's — which is why
/// [`Self::aim`] is in the gate and `hovered` is not. And the *placement* is
/// not here either, deliberately: `input::tray_drag` writes the panel's `Node`
/// directly precisely so that dragging the sheet does not rebuild it, and a
/// field here would undo that at the first pixel of the drag.
#[derive(Resource, Default)]
pub struct TrayRevision {
    /// The panel's own state: open, which tab, the filter, the sort.
    browser: super::BrowserGate,
    /// The snapshot the rows describe.
    seq: Option<u64>,
    /// Which rows are ticked — and, in an ordering, their place numbers.
    selected: Vec<ObjectId>,
    /// Where the keyboard is standing, as `(position, count)`.
    ///
    /// [`baylee_client_core::Interaction::focus_position`] rather than `aim`
    /// itself, because the two read the same `focus` and this one is `Copy`.
    aim: Option<(usize, usize)>,
    /// The arrangement being built, whole.
    ///
    /// Neither field above sees it move: `selected` is the interaction's
    /// picks, which an arrangement keeps none of, and `aim` is a position in
    /// the walk that a card nudged from the end of one pile to the end of
    /// the next can keep. A value and not a counter, for the reason the
    /// others are — it is compared, not bumped.
    arrangement: Option<baylee_client_core::arrange::Arrangement>,

    /// Which pile cards this seat is offered, and by whom
    /// ([`crate::Duel::reach_of`]): the rows' two lights, in zone order.
    ///
    /// A value of its own and not left to `seq`, because the two sets it
    /// reads are rebuilt from the interaction as well as the view, and a row
    /// drawn lit for a card the engine has stopped offering is a click that
    /// answers with a refusal.
    lit: Vec<(ObjectId, crate::Reach)>,

    /// The text-face latch, which turns every thumbnail over at once.
    faces: bool,
    /// Which filing of the card-text table this was drawn at
    /// ([`CardTexts::generation`](crate::cardtext::CardTexts::generation)).
    texts: u64,
    /// The window, rounded to whole pixels — a band a sheet is re-fitted to.
    window: (i32, i32),
    /// Set by whatever has just finished changing the sheet's **size**.
    ///
    /// The rectangle is deliberately not one of the fields above: a drag
    /// writes it on every frame and rebuilding two hundred nodes per pixel
    /// would make the sheet unusable, which is why `input::write_placement`
    /// exists at all. But a sheet built at one size and then resized *behind*
    /// the gate is drawn from numbers that are no longer true — the grid's
    /// tiles keep the width they were packed at, and the head's maximise
    /// button keeps the mark it was given. So the movement writes this when
    /// it stops, and exactly one rebuild follows.
    ///
    /// A flag rather than the rectangle itself, because what is being said is
    /// "something ended", not "here is a new value to compare": the value is
    /// already in `settings.zone_browser`, which is where the rebuild reads
    /// it from.
    stale: bool,
}

impl TrayRevision {
    /// Asks for one rebuild, at the end of a movement that resized the sheet.
    ///
    /// Called by `input::glide_the_sheet` when a maximise or a restore lands,
    /// and by `input::tray_drag` when a drag of the corner is let go of. Not
    /// during either: that is the whole point of keeping the rectangle out of
    /// the gate.
    pub fn relayout(&mut self) {
        self.stale = true;
    }
}

/// Draws the zone dialog, and only when the dialog has changed.
///
/// Two nodes rather than one subtree, and that is forced rather than chosen:
/// the veil stands at [`Z_VEIL`] and the panel at [`Z_SHEET`], with the
/// ledge's [`Z_LEDGE`] **between** them, because the shelf carries the
/// question and its answers and a question drawn dimmed is a question the
/// player is being told not to answer. A `ZIndex` orders a node among its own
/// parent's children, so the two have to be siblings of the shelf and
/// therefore two children of [`super::HudRoot`], not one wrapper.
///
/// It runs after `sync_overlay` and keeps its nodes out of that system's
/// sweep by marker ([`super::OverlayTree`]), the same bargain the shelf and
/// the drawer already have. When the overlay tears the root down — a game
/// that is over, or one that has not started — these go with it, the queries
/// below come back empty, and the gate rebuilds from the first frame there is
/// a root again.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the stores
#[allow(clippy::too_many_lines)] // One retained dialog lifecycle, including viewport preservation.
pub fn sync_tray(
    mut commands: Commands,
    duel: Res<crate::Duel>,
    mut revision: ResMut<TrayRevision>,
    mut reveal: ResMut<TrayReveal>,
    tree: TrayTree,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    texts: Res<crate::cardtext::CardTexts>,
    ui: Option<Res<UiScale>>,
    mode: Res<crate::face::FaceMode>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    (rows, mut carry): (TrayRows, ResMut<TrayCarry>),
) {
    let Ok(root) = tree.root.single() else {
        // No overlay yet. Nothing of ours can be standing either — a despawn
        // takes the descendants — so there is nothing to tear down and the
        // gate is left untouched, ready to build on the frame a root appears.
        return;
    };
    let browser = super::BrowserGate {
        sheet: super::SheetOwner::of(&duel.browser),
        ticked: duel.browser.ticked().clone(),
        filter: duel.browser.filter_field().clone(),
        typing: duel.browser.is_typing(),
        sort: duel.browser.sort(),
        descending: duel.browser.descending(),
        view: settings.zone_view,
    };
    let seq = duel.board.as_ref().map(|b| b.seq);
    let selected: Vec<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| i.selected().collect())
        .unwrap_or_default();
    let aim = duel
        .interaction
        .as_ref()
        .and_then(baylee_client_core::Interaction::focus_position);
    let arrangement = duel
        .interaction
        .as_ref()
        .and_then(baylee_client_core::Interaction::arrangement);
    let lit = pile_reach(&duel);
    // Rounded to whole pixels for `HudRevision`'s reason: a window being
    // dragged reports fractional sizes, and a gate keyed on an `f32` would
    // rebuild on a sub-pixel wobble.
    let canvas = super::scale::window_space(&windows, ui.as_deref())
        .map_or((1200, 800), |w| (w.x as i32, w.y as i32));
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: duel.view.as_ref(),
        // The browser's thumbnails are always the art, so nothing is fitted.
        widths: crate::face::Widths::of(None),
    };
    let faces_always = faces.always();
    // `!drawn` is this gate's `!tree.root.is_empty()`: the overlay can take
    // the root away under us, and a revision that still said "open, already
    // drawn" would leave the dialog missing until something else about it
    // changed.
    let drawn = !tree.panel.is_empty();
    // And whether what is drawn is a sheet that is *up*. A sheet flying into
    // the tray is not: the reading it belonged to is already gone and it is
    // on screen only because the owner asked for a way out as well as a way
    // in. This is the drawer's `open` one dialog along, and it is what stops
    // the close animation dying on its second frame — with `drawn` alone, a
    // standing sheet beside a shut browser fails this gate on every frame,
    // and the body below despawns the thing that is still moving.
    let showing = drawn && !reveal.closing;
    if revision.browser == browser
        && revision.seq == seq
        && revision.selected == selected
        && revision.aim == aim
        && revision.arrangement.as_ref() == arrangement
        && revision.lit == lit
        && revision.faces == faces_always
        && revision.texts == texts.generation()
        && revision.window == canvas
        && showing == browser.sheet.open()
        && !revision.stale
    {
        return;
    }
    // Image handles become drawable in place; their arrival must not tear
    // down the list. Preserve the viewport on selection and snapshot updates.
    let scroll_y = if revision.browser.sheet == browser.sheet
        && revision.browser.ticked == browser.ticked
        && revision.browser.filter.text() == browser.filter.text()
        && revision.browser.sort == browser.sort
        && revision.browser.descending == browser.descending
        && revision.browser.view == browser.view
    {
        tree.scroll.iter().next().map_or(0.0, |p| p.y)
    } else {
        0.0
    };
    revision.stale = false;
    revision.browser = browser.clone();
    revision.seq = seq;
    revision.selected.clone_from(&selected);
    revision.aim = aim;
    revision.arrangement = arrangement.cloned();
    revision.lit.clone_from(&lit);
    revision.faces = faces_always;
    revision.texts = texts.generation();
    revision.window = canvas;

    // Sent away rather than despawned, which is `sync_drawer`'s word for the
    // same handover: [`reveal_tray`] flies the sheet into the tray and takes
    // it off the tree at the end of that. Until then its rows are still
    // pickable, which is safe for the drawer's reason — every click is
    // re-resolved against the *current* interaction, and there is none.
    if !browser.sheet.open() {
        reveal.send_away(showing);
        return;
    }

    // What the pointer had made of each row, handed to the rows that replace
    // them ([`carry_the_rows`]): a tick is a rebuild, and the row just
    // clicked is the row under the pointer.
    carry.0 = rows
        .iter()
        .map(|(card, feel, held)| (card.object, feel.warmth, held.copied()))
        .collect();
    for entity in &tree.panel {
        commands.entity(entity).despawn();
    }
    reveal.arrives(!drawn, !browser.sheet.for_choice());

    let (Some(view), Some(statics)) = (duel.view.as_ref(), duel.statics.as_ref()) else {
        return;
    };

    let mut cards = match (ui_materials, material_assets) {
        (Some(cache), Some(assets)) => Some((cache, assets)),
        _ => None,
    };
    let mut cards = cards.as_mut().map(|(cache, assets)| UiCards {
        cache: cache.as_mut(),
        assets: assets.as_mut(),
    });
    let lang = Lang::of(&settings.lang);

    // W2: the table goes dark behind a dialog that holds the whole answer,
    // and behind no other — `Browser::dims_the_table` carries the argument
    // for why that is a narrower question than "a question opened this".
    //
    // The node is spawned whenever the *sheet* is, and it is `dim_the_table`
    // that decides how dark it is: a question answered by a second one that
    // the sheet only partly holds leaves the sheet standing with its lock
    // gone, and a veil that was spawned on the lock would vanish there
    // instead of lifting. Clear, it is one node painting nothing and
    // answering nothing.
    //
    // Kept, where one stands: a veil spawned again comes up clear until
    // `dim_the_table` paints it, and a tick in the list is no reason for the
    // table behind it to blink.
    if tree.veil.is_empty() {
        spawn_tray_veil(&mut commands, root);
    }
    // Where the sheet stands, decided by the browser so that the tray takes a
    // rectangle rather than the window and the store. `fit` is applied on
    // every build and never written back: a window briefly dragged narrow
    // must not overwrite where the player put the sheet on the screen they
    // play on. A sheet a *question* opened reads no store at all and is
    // centred — `Browser::placement` carries the measurement that says why a
    // clamp was not enough.
    let band = band_of(&windows, ui.as_deref());
    let place = if duel
        .browser
        .compact_targets(view, duel.interaction.as_ref())
    {
        compact_target_placement(
            band,
            duel.interaction
                .as_ref()
                .map_or(0, |i| i.selectable().len()),
        )
    } else {
        duel.browser.placement(band, settings.zone_browser)
    };
    let tray = spawn_tray(
        &mut commands,
        lang,
        &duel.browser,
        view,
        duel.interaction.as_ref(),
        statics,
        &mut textures,
        &assets,
        &fonts,
        &faces,
        cards.as_mut(),
        place,
        place.is_maximised(band),
        settings.zone_view,
        scroll_y,
        &lit,
    );
    commands.entity(root).add_child(tray);
}

/// The rows standing, with what the pointer has made of each.
pub type TrayRows<'w, 's> = Query<
    'w,
    's,
    (
        &'static TrayCard,
        &'static crate::ambience::Feel,
        Option<&'static bevy::picking::hover::PickingInteraction>,
    ),
>;

/// What the pointer had made of each row when [`sync_tray`] last tore the
/// rows down: its warmth and its picking state, by object.
#[derive(Resource, Default)]
pub struct TrayCarry(
    Vec<(
        ObjectId,
        f32,
        Option<bevy::picking::hover::PickingInteraction>,
    )>,
);

/// Hands each new row what the pointer had made of the row it replaces.
///
/// The dialog's flicker while choosing (report 11): every tick rebuilds the
/// rows, a row torn down and written again comes back with a fresh [`Feel`]
/// at `warmth: 0` and no picking state, and picking needs a frame to find the
/// new entity under the pointer — so the row just clicked went dark and lit
/// up again, and a tile shrank and grew back, on every click. Before
/// `ambience::feel`, which then paints the row from the warmth it carried.
pub fn carry_the_rows(
    mut commands: Commands,
    mut carry: ResMut<TrayCarry>,
    mut fresh: Query<(Entity, &TrayCard, &mut crate::ambience::Feel), Added<TrayCard>>,
) {
    if carry.0.is_empty() {
        return;
    }
    for (entity, card, mut feel) in &mut fresh {
        let Some(&(_, warmth, held)) = carry.0.iter().find(|(id, ..)| *id == card.object) else {
            continue;
        };
        feel.warmth = warmth;
        if let Some(held) = held {
            commands.entity(entity).insert(held);
        }
    }
    carry.0.clear();
}

/// Every card in a graveyard, an exile pile or a command zone that this seat
/// is offered, and by whom — [`crate::Duel::reach_of`], the predicate the
/// table lights the same cards by, so a card cannot be lit on the felt and
/// dark in the sheet.
fn pile_reach(duel: &crate::Duel) -> Vec<(ObjectId, crate::Reach)> {
    let Some(view) = duel.view.as_ref() else {
        return Vec::new();
    };
    view.graveyards
        .iter()
        .chain(&view.exile)
        .chain(&view.command)
        .flatten()
        .filter_map(|o| duel.reach_of(o.id).map(|reach| (o.id, reach)))
        .collect()
}

/// Lights a row's picture the way the hand lights a card
/// (`hud::hand`'s halo): gold for the engine's yes, indigo for this
/// client's offer to tap lands first (#242).
///
/// On the picture and not the row, because the row's own ground already
/// carries the answer being built, and a card's edge is where the hand
/// taught a player to read these two lights.
fn light_thumb(commands: &mut Commands, thumb: Entity, reach: Option<crate::Reach>) {
    let halo = match reach {
        Some(crate::Reach::Offered) => super::hand::halo(palette::ACTIVE, 1.0),
        Some(crate::Reach::Taps) => super::hand::halo(palette::REACHABLE, 0.88),
        None => return,
    };
    commands.entity(thumb).insert(halo);
}

/// What [`pile_reach`] says about one row.
fn reach_in(lit: &[(ObjectId, crate::Reach)], object: ObjectId) -> Option<crate::Reach> {
    lit.iter()
        .find_map(|(id, reach)| (*id == object).then_some(*reach))
}

/// The strip of screen the sheet is allowed into: below the report button's
/// corner ([`TOP_CLEAR`], #309), above the hand zone. The log panel is sized
/// to it as well.
///
/// One function because three places need the same answer and a band computed
/// twice is a band that can disagree with itself — the overlay places the
/// sheet in it, the drag clamps against it, and a resized window re-fits to
/// it. A window that has not been created yet answers with the size the rest
/// of the overlay falls back to.
pub(crate) fn band_of(windows: &Query<&Window>, ui: Option<&UiScale>) -> (f32, f32) {
    band_in(super::scale::window_space(windows, ui).unwrap_or(Vec2::new(1280.0, 720.0)))
}

/// [`band_of`] for a window of this size.
pub(crate) fn band_in(window: Vec2) -> (f32, f32) {
    (
        window.x,
        (window.y - TOP_CLEAR - HAND_ZONE_H).max(Placement::MIN_H),
    )
}

/// How fast the veil rises, as the rate of `1 - e^(-rate·dt)`.
///
/// Nine, against the fourteen every button in this client hovers at: 90% of
/// the way in `ln(10)/9`, about a quarter of a second. A hover answers the
/// pointer and may be quick; a veil changes the whole scene and reads as an
/// accident if it simply appears — but it has to be settled before the eye has
/// finished reading the dialog's title, and anything past a third of a second
/// is the player waiting.
///
/// It used to rise and never fall **on screen**: the dialog was a retained
/// tree that went the instant it was answered and took the veil with it, so
/// the number eased back down with nothing to draw — which was still worth
/// doing, because it is what makes the *next* question fade in from nothing
/// rather than snapping from wherever the last one stopped.
///
/// It falls on screen now. A sheet that is put away or answered outlives the
/// browser being shut, and so does its veil — [`super::TrayVeil`] is what
/// lets the dialog's teardown tell its own from the end screen's — so the
/// table lightens under a sheet that is still leaving instead of at the
/// instant it stops existing. The number did not change; what changed is
/// that there is something on screen to paint with it.
const VEIL_RATE: f32 = 9.0;

/// This dialog's veil, hung under the overlay's root.
///
/// One function and not three statements, because the marker is the whole
/// point of it. [`spawn_veil`] is called by `hud::finish` as well, so the
/// node it makes is not owned by anybody in particular; [`TrayVeil`] is what
/// says this one is the zone sheet's, and a build that spawned the veil and
/// forgot the marker would hand the end screen's darkening to this dialog's
/// teardown — which is the defect that made the marker necessary. Marking it
/// here rather than inside `spawn_veil` keeps `hud::finish` able to make one
/// that is nobody's.
fn spawn_tray_veil(commands: &mut Commands, root: Entity) {
    let veil = spawn_veil(commands);
    commands.entity(veil).insert(TrayVeil);
    commands.entity(root).add_child(veil);
}

/// The veil over the table, and the number behind it.
///
/// One full-window node, painting [`palette::TABLE_VEIL`] at whatever fraction the
/// fade has reached, answering no click at all. `Pickable::IGNORE` is the
/// whole of W2's scope: the owner asked for darkening, not for blocking, and
/// a veil that swallowed clicks would be making a claim the model does not
/// make — [`Browser::dims_the_table`] is drawn from `locked`, and even a
/// locked question leaves the board worth *pointing* at. A click that lands
/// here falls through to `input::pointer`'s "nothing interactive" branch,
/// which clears the preview, which is what a click on the table's empty felt
/// has always done.
///
/// It is the window and not [`band_of`]'s strip, because the hand zone is the
/// one thing under it a player might otherwise still reach for, and a question
/// whose every answer is in the dialog is exactly the question the hand cannot
/// answer.
///
/// It is spawned fully clear and painted by [`dim_the_table`], which runs
/// after the rebuild in the same frame: the fade lives in [`Veil`] rather than
/// on this node, and reading it here would mean a seventeenth system parameter
/// on `sync_overlay`, which already carries sixteen.
pub(super) fn spawn_veil(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            TableVeil,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(veil_at(0.0)),
            ZIndex(Z_VEIL),
            Pickable::IGNORE,
        ))
        .id()
}

/// [`palette::TABLE_VEIL`] at `lit` of its alpha.
fn veil_at(lit: f32) -> Color {
    palette::TABLE_VEIL.with_alpha(palette::TABLE_VEIL.alpha() * lit.clamp(0.0, 1.0))
}

/// Eases the veil towards where the browser says it should be, and paints it.
///
/// After `sync_overlay` deliberately, for the reason [`spawn_veil`] gives: a
/// veil spawned this frame is spawned clear, and this is what gives it its
/// colour before anything is drawn. The number itself is eased whether a veil
/// exists or not, which is what lets it fall back to nothing while there is
/// nothing on screen to fall.
///
/// `reduce_motion` takes the whole step at once, the way [`Feel`] does — the
/// veil is still drawn, it simply arrives.
pub(crate) fn dim_the_table(
    time: Res<Time>,
    duel: Res<crate::Duel>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut veil: ResMut<Veil>,
    mut nodes: Query<&mut BackgroundColor, With<TableVeil>>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    // A finished game darkens the table for the same reason a search does,
    // and it is the stronger case of the two: a dialog holds the whole
    // answer, and a game that is over has no answer left anywhere. The end
    // screen spawns a veil of its own and this is what paints it, so the
    // two surfaces that dim this table dim it by the same arithmetic at the
    // same rate rather than by two fades that could disagree.
    let target = if duel.browser.dims_the_table() || duel.ending().is_some() {
        1.0
    } else {
        0.0
    };
    let step = if still {
        1.0
    } else {
        1.0 - (-VEIL_RATE * time.delta_secs()).exp()
    };
    veil.lit += (target - veil.lit) * step;
    if (veil.lit - target).abs() < 0.001 {
        veil.lit = target;
    }
    let colour = veil_at(veil.lit);
    for mut background in &mut nodes {
        background.0 = colour;
    }
}

/// A line of a dialog's prose.
///
/// The same bracket rule the parchment sheets use — `prose::bracketed` greys
/// what a sentence says in brackets — in the dialog's own two inks. It was
/// not `overlay::slip_text`, which carried a warm shadow to lift ink off
/// parchment and greyed its asides in [`palette::SLIP_ASIDE`]: both belonged
/// to the sheet rather than to this panel, and ink on a dark ground needs no
/// shadow to be a stroke. That function went with the prompt slip in §10.2
/// step 6; `ledge::sentence` is the dialog register's version of the same
/// idea, and this one stays separate from it because a graveyard is *listed*
/// on a panel where a question is *asked* on one.
fn dialog_text(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    ink: Color,
) -> Entity {
    dialog_line(commands, fonts, text, size, ink, tf)
}

/// Question prose wraps inside the sheet and uses the shared Mana-font spans.
fn target_context_line(commands: &mut Commands, fonts: &UiFonts, text: &str) -> Entity {
    let line = crate::manaui::spawn_rich(commands, fonts, text, 11.0, palette::DIALOG_INK);
    commands
        .entity(line)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = percent(100);
            node.min_width = px(0);
            node.max_width = percent(100);
            node.flex_shrink = 0.0;
        });
    line
}

/// The size the filter box's own text is set at, in one place because the
/// caret's height is derived from it.
const FILTER_TEXT: f32 = 11.5;

/// What is inside the search box while it holds the keyboard: up to three
/// runs with a bar between two of them.
///
/// The same three runs `lobby::ui::field_runs` draws and through the same
/// door — [`TextBuffer::segments`] — because the owner asked for a box that
/// works like the lobby's, and two spellings of "where is the caret" would be
/// two things to keep in step. No glyph metrics anywhere: the row is already
/// measuring the letters, so a bar put into it as the next thing in the row
/// lands exactly where they end.
///
/// It used to be one string with `▏` stuck on the end of it, which is a caret
/// that can only ever be in one place — and it was, because the model behind
/// it was a `String` that characters were pushed onto.
///
/// [`TextBuffer::segments`]: baylee_client_core::textbuf::TextBuffer::segments
fn filter_runs(
    commands: &mut Commands,
    fonts: &UiFonts,
    browser: &baylee_client_core::Browser,
    ink: Color,
) -> Vec<Entity> {
    let field = browser.filter_field();
    let seg = field.segments();
    // Head, caret, selection, tail — with the caret on the far side of the
    // selection when that is the end the player is holding.
    let caret_at = usize::from(seg.caret_after_selection) + 1;
    let runs = [(seg.head, false), (seg.selected, true), (seg.tail, false)];
    let mut out = Vec::new();
    for (i, (text, selected)) in runs.into_iter().enumerate() {
        if i == caret_at {
            out.push(
                commands
                    .spawn((
                        Node {
                            width: px(1),
                            // Bevy lays a text node out at 1.2 times the font
                            // size, so a shorter bar would stand lower than
                            // the selection beside it and read as a fault
                            // rather than as a caret.
                            height: px(FILTER_TEXT * 1.2),
                            margin: UiRect::horizontal(px(-0.5)),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(palette::CANDLE),
                        Pickable::IGNORE,
                    ))
                    .id(),
            );
        }
        if text.is_empty() {
            continue;
        }
        let run = dialog_text(commands, fonts, text, FILTER_TEXT, ink);
        if selected {
            commands
                .entity(run)
                .insert(BackgroundColor(palette::SELECTION));
        }
        out.push(run);
    }
    out
}

/// The same line, set as a **control's own label**.
///
/// The tray's buttons are its tabs, its sort controls and the two words in
/// its foot, and all of them are built from this rather than from
/// `lobby::ui::button` — the browser is a panel of its own and shares none of
/// that widget. So the bold face has to be reachable from here too, and for
/// the same reason it is a second door rather than a flag: a card's name and
/// a tab's are both lines of this dialog, and only one of them can be
/// pressed.
fn dialog_label(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    ink: Color,
) -> Entity {
    dialog_line(commands, fonts, text, size, ink, tf_bold)
}

/// Both of the above, with the face they differ in passed in.
fn dialog_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    ink: Color,
    face: fn(&UiFonts, f32) -> TextFont,
) -> Entity {
    let line = commands
        .spawn((
            Text::default(),
            face(fonts, size),
            TextColor(ink),
            // Every line on this dialog stands in a band of a fixed height,
            // so a line that wrapped would have its second half cut off by
            // the row it is in. Too long is clipped at the end instead, which
            // at least says which card it is.
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Pickable::IGNORE,
        ))
        .id();
    for (run, aside) in baylee_client_core::prose::bracketed(text) {
        let span = commands
            .spawn((
                TextSpan::new(run.to_string()),
                face(fonts, size),
                TextColor(if aside { palette::DIALOG_SOFT } else { ink }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    line
}

/// Puts a line of [`dialog_text`] in a box that actually clips it.
///
/// A `Node`'s `Overflow` clips its **descendants**, not its own glyphs:
/// `CalculatedClip` is propagated down by `update_clipping_system`, so a
/// `Text` entity carrying `Overflow::clip()` clips nothing at all, having no
/// children to clip. Both columns of a row had exactly that, and both had a
/// comment saying "too long is clipped at the end instead" — which was true
/// of the intent and false of the drawing. A German type line ran straight
/// out of its measure and printed underneath the zone badge:
/// `Legendäre Kreatur — Mensch, Verbündeter` is 32% wider than the
/// `Legendary Planeswalker — Aminatou` [`TRAY_TYPE_W`] was measured from, and
/// a client is drawn in whichever language the player picked.
///
/// So the clip goes on a box and the line goes inside it. `flex_shrink: 0`
/// on the line is what makes the clip do anything: a line allowed to shrink
/// gets a narrower *box* and lays its glyphs out just the same, so the
/// overrun survives in a shorter node and the clip rect never bites.
fn clipped(commands: &mut Commands, line: Entity, box_node: Node) -> Entity {
    commands.entity(line).insert(Node {
        flex_shrink: 0.0,
        ..default()
    });
    let clip = commands
        .spawn((
            Node {
                overflow: Overflow::clip(),
                ..box_node
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(clip).add_child(line);
    clip
}

/// One of the head's window buttons.
///
/// Both of them differ by exactly two things — the marker component that says
/// which click this is, and the mark drawn in it — so they are one function
/// rather than two literals that agree for now. The marker is inserted by the
/// caller because it is the only part a shared body cannot carry, and writing
/// the pair out twice is how a `Feel`, a radius or an ink ends up on one of
/// them and not the other.
///
/// The mark comes out of the **icon font**, not the text face: Alegreya Sans
/// carries neither a window rule nor a frame, which is why the minimise
/// button drew as a thin bar for one build when it was still a `✕`.
fn head_button(commands: &mut Commands, fonts: &UiFonts, mark: char) -> Entity {
    commands
        .spawn((
            UiButton,
            Node {
                width: px(HEAD_BTN),
                height: px(HEAD_BTN),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::new(palette::DIALOG),
            children![(
                Text::new(mark.to_string()),
                icon_tf(fonts, HEAD_BTN_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// Small complete offers keep the board visible and all rows directly reachable.
fn compact_target_placement(band: (f32, f32), count: usize) -> Placement {
    let width = 640.0_f32.min((band.0 - 24.0).max(0.0));
    let rows = f32::from(u16::try_from(count).unwrap_or(u16::MAX));
    let height = (150.0 + rows * TRAY_ROW_H).min((band.1 - 24.0).max(0.0));
    Placement {
        left: (band.0 - width) / 2.0,
        top: (band.1 - height) / 2.0,
        width,
        height,
    }
}

/// What a grid needs that a row does not: the measure it shares out, and
/// whether the runs are headed.
///
/// A struct because [`spawn_grid`] would otherwise take eleven arguments, and
/// these four belong together — they are the answers to "how wide" and "how
/// many piles", which is the whole of what makes a grid different from a
/// list.
#[derive(Clone, Copy)]
struct GridCtx<'a> {
    browser: &'a Browser,
    interaction: Option<&'a baylee_client_core::Interaction>,
    view: &'a PlayerView,
    statics: &'a GameStatic,
    fonts: &'a UiFonts,
    /// The width the tiles share, which is the sheet less the list's gutters.
    measure: f32,
    /// Whether to head each pile's run with its name.
    headed: bool,
    /// Which rows' cards this seat is offered, and by whom ([`pile_reach`]).
    lit: &'a [(ObjectId, crate::Reach)],
}

mod arrange;
mod footer;
mod rows;
mod spawn;
#[cfg(test)]
mod tests;
