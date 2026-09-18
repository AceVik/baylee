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
//! `docs/redesign-proposal.md` §1.3 draws the line: **parchment is a sheet
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
use baylee_client_core::browser::{BrowseRow, BrowseZone, Browser, Names, ViewMode, grid_across};

/// Opening progress survives filter, tab and card-art rebuilds.
#[derive(Resource, Default)]
pub struct TrayReveal(f32);

/// A restrained entrance, with readable text and no animation on filtering.
pub fn reveal_tray(
    time: Res<Time>,
    duel: Res<Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut reveal: ResMut<TrayReveal>,
    mut panels: Query<&mut UiTransform, With<TrayPanel>>,
) {
    if !duel.browser.is_open() {
        reveal.0 = 0.0;
        return;
    }
    reveal.0 = if prefs.all().reduce_motion {
        1.0
    } else {
        (reveal.0 + time.delta_secs() / 0.20).min(1.0)
    };
    let eased = 1.0 - (1.0 - reveal.0).powi(3);
    for mut transform in &mut panels {
        transform.scale = Vec2::splat(0.965 + 0.035 * eased);
        transform.translation.y = px((1.0 - eased) * 12.0);
    }
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
/// Eight and a half until the rows grew on 14.09.2026 (see [`TRAY_THUMB_W`]).
/// Seven and a half is what keeps the sheet close to the height it opened at
/// before: a taller row spent entirely on more sheet would have put the
/// default at 768 of the 850 the band has at 1738, which is a dialog that
/// reads as a screen.
#[cfg(test)]
const TRAY_ROWS: f32 = 8.5;

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
    /// Card art that has arrived since the last build.
    arrivals: u64,
    /// The text-face latch, which turns every thumbnail over at once.
    faces: bool,
    /// How many card texts have been fetched.
    texts: usize,
    /// The window, rounded to whole pixels — a band a sheet is re-fitted to.
    window: (i32, i32),
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
pub fn sync_tray(
    mut commands: Commands,
    duel: Res<crate::Duel>,
    mut revision: ResMut<TrayRevision>,
    tree: TrayTree,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
) {
    let Ok(root) = tree.root.single() else {
        // No overlay yet. Nothing of ours can be standing either — a despawn
        // takes the descendants — so there is nothing to tear down and the
        // gate is left untouched, ready to build on the frame a root appears.
        return;
    };
    let browser = super::BrowserGate {
        open: duel.browser.is_open(),
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
    // Rounded to whole pixels for `HudRevision`'s reason: a window being
    // dragged reports fractional sizes, and a gate keyed on an `f32` would
    // rebuild on a sub-pixel wobble.
    #[allow(clippy::cast_possible_truncation)]
    let canvas = windows
        .single()
        .map_or((1200, 800), |w| (w.width() as i32, w.height() as i32));
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: duel.view.as_ref(),
    };
    let faces_always = faces.always();
    // `!drawn` is this gate's `!tree.root.is_empty()`: the overlay can take
    // the root away under us, and a revision that still said "open, already
    // drawn" would leave the dialog missing until something else about it
    // changed.
    let drawn = !tree.panel.is_empty();
    if revision.browser == browser
        && revision.seq == seq
        && revision.selected == selected
        && revision.aim == aim
        && revision.arrivals == textures.epoch()
        && revision.faces == faces_always
        && revision.texts == texts.len()
        && revision.window == canvas
        && drawn == browser.open
    {
        return;
    }
    revision.browser = browser.clone();
    revision.seq = seq;
    revision.selected.clone_from(&selected);
    revision.aim = aim;
    revision.arrivals = textures.epoch();
    revision.faces = faces_always;
    revision.texts = texts.len();
    revision.window = canvas;

    for entity in tree.panel.iter().chain(tree.veil.iter()) {
        commands.entity(entity).despawn();
    }

    let (Some(view), Some(statics)) = (duel.view.as_ref(), duel.statics.as_ref()) else {
        return;
    };
    if !browser.open {
        return;
    }

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
    let veil = spawn_veil(&mut commands);
    commands.entity(root).add_child(veil);
    // Where the sheet stands, decided by the browser so that the tray takes a
    // rectangle rather than the window and the store. `fit` is applied on
    // every build and never written back: a window briefly dragged narrow
    // must not overwrite where the player put the sheet on the screen they
    // play on. A sheet a *question* opened reads no store at all and is
    // centred — `Browser::placement` carries the measurement that says why a
    // clamp was not enough.
    let place = duel
        .browser
        .placement(band_of(&windows), settings.zone_browser);
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
        settings.zone_view,
    );
    commands.entity(root).add_child(tray);
}

/// The strip of screen the sheet is allowed into: below the seat tabs and the
/// phase rail, above the hand zone.
///
/// One function because three places need the same answer and a band computed
/// twice is a band that can disagree with itself — the overlay places the
/// sheet in it, the drag clamps against it, and a resized window re-fits to
/// it. A window that has not been created yet answers with the size the rest
/// of the overlay falls back to.
pub(crate) fn band_of(windows: &Query<&Window>) -> (f32, f32) {
    let (w, h) = windows
        .single()
        .map_or((1280.0, 720.0), |window| (window.width(), window.height()));
    (w, (h - EDGE - HAND_ZONE_H).max(Placement::MIN_H))
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
/// It rises and never falls on screen: the dialog is a retained tree and goes
/// the instant it is answered, so the veil goes with it. The number still eases
/// back down with nothing to draw, which is what makes the *next* question fade
/// in from nothing rather than snapping from wherever the last one stopped.
const VEIL_RATE: f32 = 9.0;

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

/// The zone browser: a dialog over the table, in the middle of it.
///
/// Centred rather than pinned to a corner, because that is where a stack of
/// cards goes when somebody puts one down on a real table — and a dark panel
/// rather than parchment, for the reason the module doc gives.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the stores
#[allow(clippy::too_many_lines)] // head, tabs, controls, list and footer are one build
pub(super) fn spawn_tray(
    commands: &mut Commands,
    lang: Lang,
    browser: &Browser,
    view: &PlayerView,
    interaction: Option<&baylee_client_core::Interaction>,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
    place: Placement,
    mode: ViewMode,
) -> Entity {
    // The catalog reaching the panel's own decisions, which is the half
    // `Browser` cannot do for itself: it decides in `baylee-client-core`,
    // which links neither the card registry nor the gateway's text. Bound
    // here rather than handed out by a constructor the way
    // [`crate::cardart::registry`] is, because this one closes over the view
    // and the texts and so has nowhere to live but the call.
    let shown =
        |object: &baylee_view::PublicObject| Some(crate::face::name_of(object, view, faces.texts));
    let rows = browser.rows(view, interaction, Names { shown: &shown });
    // The sheet's own cards are on no table and in no board model — a library
    // search lists a hundred that nothing else is drawing — so the texture
    // cache is told about them here or by nobody. Before this, the rows a
    // player was reading were the oldest thing in the cache and the first a
    // fetch would have thrown away.
    let on_the_sheet: Vec<baylee_client_core::images::ImageKey> =
        rows.iter().filter_map(|row| row.art).collect();
    textures.touch_visible(&on_the_sheet);
    // The question *this* sheet answers, which is not every question there
    // might be: a graveyard opened by hand while the engine asks about the
    // battlefield holds none of the answer, and grew a tally and a Confirm for
    // it anyway. `Browser::answers_here` carries the whole argument, including
    // why the prompt slip reads the same predicate and draws no second
    // Confirm behind this one. Everything else on the sheet — the rows, the
    // place numbers — still takes the interaction whole: a row is drawn as
    // selected because it *is*, whatever surface the send belongs to.
    let answering = browser
        .answers_here(interaction)
        .then_some(interaction)
        .flatten();
    // The band: the whole window between its top edge and the hand zone,
    // painting nothing and answering no click. It is the coordinate space the
    // sheet is placed in, which is what makes a remembered position mean the
    // same thing on two screens with different amounts of HUD above and below.
    //
    // Five, above [`spawn_veil`]'s three and the prompt slip's four. The
    // overlay's whole order is stated in one place — `overlay::sync_overlay`'s
    // doc — because a `ZIndex` is local to a parent's children and two
    // siblings that share one are settled by the order they were spawned in,
    // which is how the seat bars once came to be drawn through this dialog.
    let frame = commands
        .spawn((
            TrayBand,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(EDGE),
                bottom: px(HAND_ZONE_H),
                ..default()
            },
            ZIndex(Z_SHEET),
            Pickable::IGNORE,
        ))
        .id();
    let panel = commands
        .spawn((
            TrayPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(place.left),
                top: px(place.top),
                width: px(place.width),
                height: px(place.height),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(TRAY_RADIUS)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            sheet_shadow(),
        ))
        .id();
    commands.entity(frame).add_child(panel);

    // ---- the head: what this is, what it is showing, and what to type ----
    //
    // It carries the panel's own radius at the top, less the one pixel of
    // border it sits inside — the same arithmetic and the same reason as
    // [`super::sheet_surface`], which is the other place a child has to be
    // concentric with the corner it is drawn in.
    //
    // Without it the panel's two top corners were square. `Overflow::clip()`
    // clips **rectangularly** in `bevy_ui`, so a square child inside a
    // rounded parent paints over the parent's corner *and* over the arc of
    // its border, and what is left is two straight lines that stop short of
    // each other. Measured in the running client at 3008 x 1630 before the
    // change: the fill in the corner read `DIALOG_LIT` (38, 33, 25) — the
    // head's colour, not the panel's (28, 25, 19) — and the 1 px line was
    // absent over exactly the 14 px of the radius. The panel's *bottom*
    // corners, where no child reaches, drew a clean arc the whole time,
    // which is what says the radius was never the thing that was wrong.
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(TRAY_HEAD_GAP),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_HEAD_PAD)),
                border: UiRect::bottom(px(1)),
                border_radius: BorderRadius::top(px(TRAY_HEAD_R)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();

    // The title row answers the pointer, because it is what a drag takes hold
    // of. Everything inside it that is not a control keeps `Pickable::IGNORE`,
    // so a press on the bare row is a press on the row itself.
    //
    // **The tabs are in it, and the word "Zonen" is gone.** The owner asked
    // for both on 14.09.2026 — the chips say what the panel is far better
    // than a label repeating the name of the thing the player just opened,
    // and a title bar with nothing but a title in it is a row of air. They
    // were deliberately *out* of this row before, because dragging a tab
    // sideways would have carried the sheet with it; that is now settled
    // where the `✕` already settles it — `tray_drag` lets the specific
    // control claim the press before the row it stands on does.
    let title_row = commands
        .spawn((
            TrayGrip,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                height: px(TRAY_TITLE_H),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    // The way out. Square, so the cross has a centre to sit in, and with a
    // `Feel`, because every other button in this client breathes.
    let close = commands
        .spawn((
            TrayClose,
            Button,
            Node {
                width: px(22),
                height: px(22),
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
                // The icon font's own cross. The text face has no U+2715 —
                // it was Inter and is Alegreya Sans, and neither does — which
                // is why the button drew as a thin bar for one build.
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(fonts, 12.0),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id();
    // ---- the zone tabs, "All" first ----
    //
    // They take the row's slack and the `✕` keeps its 22 px, which is why the
    // tabs grow and the close button does not. `min_width` of zero is the
    // half a flex row always needs: without it a row of eight piles refuses
    // to shrink below the width of its own chips and pushes the `✕` off the
    // sheet.
    let tabs = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                height: px(TRAY_TAB_H),
                overflow: Overflow::clip(),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // "All" carries no count: a sum of a graveyard, a stack and a reveal is a
    // number about nothing. Its box is ticked when no other one is, which is
    // the empty set's meaning drawn rather than a fourth state
    // ([`Browser::shows_every_zone`]).
    //
    // A question that lives in one zone pins the tab to it (W3): nothing in
    // the row is a button then, "All" included.
    let pinned = browser.locked();
    let live = pinned.is_none();
    let mut chips = vec![spawn_tab(
        commands,
        fonts,
        None,
        Phrase::BrowseAll.text(lang).to_string(),
        browser.shows_every_zone(),
        live,
    )];
    for zone in browser.zones(view) {
        chips.push(spawn_tab(
            commands,
            fonts,
            Some(zone),
            zone_label(lang, zone, view, statics),
            browser.is_ticked(zone) || pinned == Some(zone),
            live,
        ));
    }
    commands.entity(tabs).add_children(&chips);
    commands.entity(title_row).add_children(&[tabs, close]);

    // ---- what is typed, how it is sorted, and how much is answered ----
    //
    // An ordering has no filter to offer — the panel is the answer being
    // assembled, and narrowing it would hide places in it — so the row says
    // what to do instead. Everywhere else this is a field: empty and unfocused
    // it shows what it is for, focused it shows a caret, and either way it is
    // the thing a player clicks to search the pile they are looking at.
    let ordering = interaction.is_some_and(baylee_client_core::Interaction::is_ordering);
    let typing = browser.is_typing();
    // A field being typed into draws itself out of its own segments below; a
    // field at rest is one line saying what it holds or what it is for.
    let hint = if ordering {
        Some(Phrase::BrowseOrderHint.text(lang).to_string())
    } else if typing {
        None
    } else if browser.filter().trim().is_empty() {
        Some(Phrase::BrowseFilter.text(lang).to_string())
    } else {
        Some(format!("\u{201c}{}\u{201d}", browser.filter()))
    };
    let controls = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                height: px(TRAY_CTRL_H),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let said = typing || !browser.filter().trim().is_empty();
    let ink = if said {
        palette::DIALOG_INK
    } else {
        palette::DIALOG_SOFT
    };
    let filter_text: Vec<Entity> = match &hint {
        Some(words) => vec![dialog_text(commands, fonts, words, 11.5, ink)],
        None => filter_runs(commands, fonts, browser, ink),
    };
    // The magnifier the deck builder's box wears, in this register's ink. It
    // is what says the box is a *search* before a word has been typed into
    // it, and it is deliberately the quiet ink even while the field holds the
    // keyboard: it is a label on the box, not part of what is written in it.
    let lens = commands
        .spawn((
            Text::new(glyph::MAGNIFIER.to_string()),
            icon_tf(fonts, 10.5),
            TextColor(palette::DIALOG_SOFT),
            Node {
                flex_shrink: 0.0,
                margin: UiRect::right(px(TRAY_GAP - 4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // A sunk field: the ring that says where the typing goes is the border
    // turning candle, not a second fill.
    let filter_line = commands
        .spawn((
            TrayFilter,
            Button,
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                height: percent(100),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(9)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(if typing {
                palette::CANDLE
            } else {
                palette::DIALOG_LINE
            }),
            Feel::new(palette::DIALOG),
        ))
        .id();
    // The gear lives *inside* the box, which is what says it is about what
    // the box holds. It stays lit while the builder is open, because the
    // builder has no frame of its own to say so — it is a mode of this field
    // and not a window.
    let building = browser.builder().is_some();
    let gear = commands
        .spawn((
            TrayGear,
            Button,
            Node {
                width: px(TRAY_CTRL_H - 8.0),
                height: px(TRAY_CTRL_H - 8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                // `auto` on the left is what puts it at the far end of the
                // box rather than beside the text: the runs are sized to the
                // letters in them, so a fixed margin would walk the gear
                // along as the player typed.
                margin: UiRect::left(Val::Auto),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if building {
                palette::CANDLE_WASH
            } else {
                Color::NONE
            }),
            Feel::tinting_to(
                if building {
                    palette::CANDLE_WASH
                } else {
                    Color::NONE
                },
                palette::CANDLE_WASH_LIT,
            ),
        ))
        .id();
    let cog = commands
        .spawn((
            Text::new(glyph::GEAR.to_string()),
            icon_tf(fonts, 11.0),
            TextColor(if building {
                palette::CANDLE
            } else {
                palette::DIALOG_SOFT
            }),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(gear).add_child(cog);
    commands.entity(filter_line).add_child(lens);
    commands.entity(filter_line).add_children(&filter_text);
    commands.entity(filter_line).add_child(gear);
    // A library is a hundred cards and a long graveyard is thirty, so "look
    // through this pile" is not a question the pile's own order answers on
    // its own. The key and the direction are two buttons because they are two
    // questions, and the arrow says which way the current one runs rather
    // than being a third state of the key.
    let sort_key = spawn_control(
        commands,
        fonts,
        TraySort { reverse: false },
        browser.sort().label().text(lang),
        9.0,
    );
    let sort_dir = spawn_control(
        commands,
        fonts,
        TraySort { reverse: true },
        if browser.descending() {
            "\u{2193}"
        } else {
            "\u{2191}"
        },
        8.0,
    );
    // And after them, the three shapes the same rows can be drawn in. They
    // sit at the right end so that every "how it is shown" control is one
    // cluster and the search field keeps the growing left — and they are
    // three buttons rather than a fourth cycling one, for the reason
    // [`super::TrayView`] gives: a sort key is a ring of equivalent answers
    // and a view is a shape you are looking at.
    let views = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                height: percent(100),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    let segments: Vec<Entity> = ViewMode::ALL
        .into_iter()
        .map(|each| spawn_view(commands, fonts, each, each == mode))
        .collect();
    commands.entity(views).add_children(&segments);
    commands
        .entity(controls)
        .add_children(&[filter_line, sort_key, sort_dir, views]);
    // The tally. The engine names a minimum and a maximum, so the dialog can
    // say how far along the answer is — and a panel with no question in it (a
    // graveyard opened by hand) says nothing rather than "0 of 0".
    if let Some((min, max)) = answering.and_then(baylee_client_core::Interaction::bounds) {
        let chosen = answering.map_or(0, baylee_client_core::Interaction::declared);
        let words = if min == max {
            Phrase::BrowseTallyExact.fill(lang, &[&chosen.to_string(), &max.to_string()])
        } else {
            Phrase::BrowseTallyUpTo.fill(lang, &[&chosen.to_string(), &max.to_string()])
        };
        let tally = dialog_text(commands, fonts, &words, 10.5, palette::DIALOG_SOFT);
        commands.entity(controls).add_child(tally);
    }

    // Two rows, where it was three: the tabs went up into the title row —
    // and a third when the gear is open, which is the builder. It sits under
    // the controls rather than over the list, because it is what the box in
    // that row holds: a panel floating over the cards would be a second
    // window, and this is a mode of the field above it.
    commands.entity(head).add_children(&[title_row, controls]);
    if let Some(panel) = browser.builder() {
        let built = crate::filterui::build(
            commands,
            fonts,
            panel,
            baylee_client_core::cardquery::Surface::ZONE,
            lang,
            crate::filterui::Register::TRAY,
        );
        commands.entity(head).add_child(built);
    }

    // ---- the list ----
    //
    // It scrolls, and it is the list rather than the panel: the tabs, the
    // search field and the sort control have to stay where they are while a
    // hundred-card library is scrolled past them. `Pickable` and not
    // `Pickable::IGNORE`, because the picking backend is what turns a wheel
    // into the `Pointer<Scroll>` a scrolling node listens for — the same
    // reason `dev-control` has to put the pointer over a list before it can
    // send one.
    //
    // `min_height: px(0)` beside the `flex_grow`, and it is load-bearing: a
    // flex item's default `min-height` is `auto`, so a hundred-card library
    // would size the column to its own content, push the sheet past the
    // explicit height it was given, and hand the overflow — including the
    // resize corner — to `Overflow::clip`.
    let list = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            // The two halves the sentence above only claimed. An overflow
            // clips and nothing else — Bevy moves the content when
            // `ScrollPosition` changes and nothing changes it on its own —
            // so until `hud::scrolls` existed this list ended at the bottom
            // of the sheet with the rest of the library behind it, and the
            // wheel that should have reached it zoomed the table.
            super::Scrolls,
            ScrollPosition::default(),
        ))
        .id();
    if rows.is_empty() {
        let empty = commands
            .spawn((
                Node {
                    padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_ROW_PAD * 2.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let words = dialog_text(
            commands,
            fonts,
            Phrase::BrowseEmpty.text(lang),
            11.5,
            palette::DIALOG_SOFT,
        );
        commands.entity(empty).add_child(words);
        commands.entity(list).add_child(empty);
    }
    match mode {
        ViewMode::Detailed | ViewMode::Large => {
            for row in &rows {
                let node = if mode == ViewMode::Large {
                    spawn_big_row(
                        commands, lang, row, view, statics, textures, assets, fonts, faces,
                        &mut cards,
                    )
                } else {
                    spawn_row(
                        commands, lang, row, view, statics, textures, assets, fonts, faces,
                        &mut cards,
                    )
                };
                commands.entity(list).add_child(node);
            }
        }
        ViewMode::Grid => spawn_grid(
            commands,
            list,
            lang,
            &rows,
            GridCtx {
                view,
                statics,
                fonts,
                // The measure the tiles share: the sheet's own width less
                // the gutter the list keeps on both sides. The panel's
                // border is inside that width already — it is a `border`,
                // not a margin — so it is not subtracted a second time.
                measure: place.width - 2.0 * TRAY_SIDE,
                // Headed runs only when the panel is showing more than one
                // pile: a tile cannot say which zone it came from, and the
                // list view says it in a badge on every row. With one tab
                // ticked the tabs above have already said it, and a heading
                // repeating that tab would be a line of chrome over every
                // grid the panel ever draws.
                headed: rows
                    .iter()
                    .map(|row| row.zone)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    > 1,
            },
            textures,
            assets,
            &mut cards,
        ),
    }

    // ---- the footer ----
    let foot = spawn_footer(commands, fonts, lang, answering);

    // The corner, in the same shape and the same place the card preview's is:
    // one handle, bottom right, both axes. A second handle on every edge is
    // eight more hit targets for a gesture nobody makes on a dialog.
    //
    // Drawn only on a sheet the player arranged. A sheet a *question* opened
    // is centred and reads no stored rectangle, so `input::tray_drag` returns
    // before it ever reaches this handle — and then the rule the pinned tabs
    // and the unlit Confirm already obey applies here too: a control that
    // lights under the pointer and refuses the gesture is worse than no
    // control at all.
    let corner = (!browser.for_choice()).then(|| {
        commands
            .spawn((
                TrayResize,
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(4),
                    bottom: px(4),
                    width: px(22),
                    height: px(22),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                // `Feel::new` shades towards white and **keeps the alpha**, so
                // a control resting at nothing is lifted to a brighter nothing
                // and never answers the pointer. The hot end is stated here
                // for exactly the reason [`Feel::hot`] exists.
                Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
                children![(
                    Text::new(glyph::EXPAND.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(palette::DIALOG_SOFT),
                    Pickable::IGNORE,
                )],
            ))
            .id()
    });

    commands.entity(panel).add_children(&[head, list]);
    if let Some(foot) = foot {
        commands.entity(panel).add_child(foot);
    }
    if let Some(corner) = corner {
        commands.entity(panel).add_child(corner);
    }
    frame
}

/// The footer, or nothing at all when there is no question to answer.
///
/// Two buttons, and both of them send what [`PromptAction::Confirm`] sends,
/// which is the whole shape of §6's footer: **Confirm is lit only when the
/// answer is complete, and Cancel is drawn only when the minimum is zero.**
/// There is no cancel on the wire — a question that will take an empty answer
/// is answered by sending one, and a question that will not has no way out to
/// offer, so a dialog that drew the button anyway would be promising what the
/// engine cannot deliver.
fn spawn_footer(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    interaction: Option<&baylee_client_core::Interaction>,
) -> Option<Entity> {
    let it = interaction?;
    let (min, _max) = it.bounds()?;
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_FOOT_PAD)),
                border: UiRect::top(px(1)),
                // The head's radius at the other end, and for the same reason
                // — this band reaches the panel's bottom two corners exactly
                // as the head reaches the top two. It is drawn only while a
                // question is standing, which is why the square corner was
                // seen at the top first.
                border_radius: BorderRadius::bottom(px(TRAY_HEAD_R)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();

    // Lit only when the answer is complete. An unlit Confirm is not a button
    // at all — no `Button`, no `Feel`, `Pickable::IGNORE` — for the reason a
    // pinned zone tab is not one: a control that lights under the pointer and
    // then refuses the click is worse than one that never invited it.
    let ready = it.can_confirm();
    let confirm = commands
        .spawn((
            Node {
                height: px(TRAY_FOOT_H),
                padding: UiRect::horizontal(px(18)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if ready {
                palette::CANDLE
            } else {
                palette::DIALOG
            }),
            BorderColor::all(if ready {
                palette::CANDLE
            } else {
                palette::DIALOG_LINE
            }),
        ))
        .id();
    let words = dialog_label(
        commands,
        fonts,
        Phrase::BrowseConfirm.text(lang),
        13.0,
        if ready {
            palette::DIALOG
        } else {
            palette::DIALOG_SOFT
        },
    );
    commands.entity(confirm).add_child(words);
    if ready {
        commands.entity(confirm).insert((
            Button,
            PromptButton {
                action: PromptAction::Confirm,
            },
            Feel::new(palette::CANDLE),
        ));
    } else {
        commands.entity(confirm).insert(Pickable::IGNORE);
    }
    commands.entity(foot).add_child(confirm);

    if min == 0 {
        let out = dialog_label(
            commands,
            fonts,
            Phrase::BrowseNone.text(lang),
            13.0,
            palette::DIALOG_SOFT,
        );
        let cancel = commands
            .spawn((
                TrayNone,
                Button,
                Node {
                    height: px(TRAY_FOOT_H),
                    padding: UiRect::horizontal(px(14)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                // A ghost button that fills in under the pointer rather than
                // one that lightens: see the resize corner for why a rest of
                // `Color::NONE` has to state its hot end.
                Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
            ))
            .id();
        commands.entity(cancel).add_child(out);
        commands.entity(foot).add_child(cancel);
    }
    Some(foot)
}

/// One zone tab: a box, and the pile's name beside it.
///
/// `live` is false while a question has pinned a tab — every other one is
/// still drawn, at the weight of something that is not a control, and none of
/// them is a button. Drawn and not hidden, because a tab that vanished would
/// be saying the graveyard is empty, and what is true is that it is no part
/// of *this* question.
///
/// **The state is in the box and not under the chip.** It was a candle fill
/// across the whole tab, which is a thing one chip at a time can say; the
/// owner asked on 14.09.2026 for a checkbox at the start of each name so that
/// ticking several merges them, and three candle chips in a row would read as
/// three panels rather than as one list. So the chip keeps the panel's own
/// dark and the box carries the accent — the same box, the same candle and
/// the same tick the rows in the list below already use, one size down.
fn spawn_tab(
    commands: &mut Commands,
    fonts: &UiFonts,
    zone: Option<BrowseZone>,
    label: String,
    ticked: bool,
    live: bool,
) -> Entity {
    // A chip outside the question gets no surface at all and the quieter ink
    // on top: the dialog already means "a different kind of sentence" by that
    // grey, which is exactly what a tab outside the question is. It also gets
    // **no box**, for the reason a row the question will not take draws none:
    // a box that cannot be ticked is an invitation that will be refused.
    let dead = !live && !ticked;
    let (fill, ink) = if dead {
        (Color::NONE, palette::DIALOG_SOFT)
    } else {
        (palette::DIALOG, palette::DIALOG_INK)
    };
    let text = dialog_label(commands, fonts, &label, 11.0, ink);
    let tab = commands
        .spawn((
            TrayTab { zone },
            Node {
                height: px(TRAY_TAB_H),
                padding: UiRect::horizontal(px(8)),
                align_items: AlignItems::Center,
                column_gap: px(6),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
        ))
        .id();
    if !dead {
        let mark = spawn_tab_box(commands, fonts, ticked);
        commands.entity(tab).add_child(mark);
    }
    if live {
        commands.entity(tab).insert((Button, Feel::new(fill)));
    } else {
        // No `Button` and no `Feel` either: a control that lights under the
        // pointer and then refuses the click is worse than one that never
        // invited it.
        commands.entity(tab).insert(Pickable::IGNORE);
    }
    commands.entity(tab).add_child(text);
    tab
}

/// The box at the start of a zone's name.
///
/// The list's own box ([`spawn_row`]) one size down and with the same four
/// decisions made the same way: candle under a tick, a hollow outline under
/// none, the tick out of the **icon** face because neither Inter nor Alegreya
/// Sans has U+2713, and `Pickable::IGNORE` so the box never takes the press
/// meant for the chip it sits in — the trap `a-label-swallows-the-hover`
/// names.
///
/// A box on a pinned tab is drawn and is not a control — the chip around it
/// carries that — and its tick stays candle, because the question really has
/// ticked that zone and a grey tick would be saying something else.
fn spawn_tab_box(commands: &mut Commands, fonts: &UiFonts, ticked: bool) -> Entity {
    let (fill, edge) = if ticked {
        (palette::CANDLE, palette::CANDLE)
    } else {
        (Color::NONE, palette::DIALOG_SOFT)
    };
    let mark = commands
        .spawn((
            Node {
                width: px(TRAY_TAB_BOX),
                height: px(TRAY_TAB_BOX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Pickable::IGNORE,
        ))
        .id();
    if ticked {
        let ink = commands
            .spawn((
                Text::new(glyph::CHECK.to_string()),
                icon_tf(fonts, 7.5),
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(mark).add_child(ink);
    }
    mark
}

/// One of the three view segments.
///
/// Icons and not words, and that is arithmetic rather than taste: three
/// labels beside the sort key would leave the search field about 140 px wide
/// on a sheet at its 356-pixel floor, which is a search box that can show a
/// card name and nothing a player is typing. There are no words behind them
/// either, which is the honest half: an icon with nothing behind it cannot be
/// named by a tooltip, a keyboard map or a reader, and [`ViewMode::name`]
/// says why the translated labels are written when something first asks
/// rather than now.
///
/// The three are one strip with one border round it rather than three
/// buttons in a row: they are a single question with three answers, and a
/// chosen segment says so by being the lit one. The unchosen two rest at
/// nothing, so the strip reads as one control with a mark in it.
fn spawn_view(commands: &mut Commands, fonts: &UiFonts, mode: ViewMode, current: bool) -> Entity {
    let (fill, ink) = if current {
        (palette::DIALOG_LIT, palette::CANDLE)
    } else {
        (Color::NONE, palette::DIALOG_SOFT)
    };
    let mark = match mode {
        ViewMode::Detailed => glyph::VIEW_ROWS,
        ViewMode::Large => glyph::VIEW_BIG,
        ViewMode::Grid => glyph::VIEW_GRID,
    };
    let icon = commands
        .spawn((
            Text::new(mark.to_string()),
            icon_tf(fonts, 10.0),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let button = commands
        .spawn((
            super::TrayView { mode },
            Button,
            Node {
                width: px(26),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(fill),
            // Both ends stated, because `Feel::new` shades towards white and
            // **keeps the alpha**: an unchosen segment rests at nothing and
            // would be lifted to a brighter nothing, which is a control that
            // never answers the pointer. The resize corner has the same note
            // for the same reason.
            if current {
                Feel::new(palette::DIALOG_LIT)
            } else {
                Feel::rising_to(Color::NONE, palette::DIALOG_LIT)
            },
        ))
        .id();
    commands.entity(button).add_child(icon);
    button
}

/// One control in the head's bottom row: the sort key, or the arrow beside it.
fn spawn_control<C: Component>(
    commands: &mut Commands,
    fonts: &UiFonts,
    marker: C,
    label: &str,
    pad: f32,
) -> Entity {
    let text = dialog_label(commands, fonts, label, 11.0, palette::DIALOG_INK);
    let button = commands
        .spawn((
            marker,
            Button,
            Node {
                height: percent(100),
                padding: UiRect::horizontal(px(pad)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::new(palette::DIALOG),
        ))
        .id();
    commands.entity(button).add_child(text);
    button
}

/// One row of the list: a checkbox, a thumbnail, the name, the cost in pips,
/// the type line and the pile it is in.
#[allow(clippy::too_many_arguments)] // the row, the view, and the stores
#[allow(clippy::too_many_lines)] // six columns in one build
fn spawn_row(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: &mut Option<&mut UiCards<'_>>,
) -> Entity {
    let slot = row_slot(commands, row, TRAY_ROW_H, TRAY_ROW_PAD);

    let mark = spawn_mark(commands, fonts, row, TRAY_BOX);
    commands.entity(slot).add_child(mark);

    let thumb = spawn_thumb(
        commands,
        lang,
        row,
        statics,
        textures,
        assets,
        fonts,
        cards,
        TRAY_THUMB_W,
    );
    commands.entity(slot).add_child(thumb);

    // ---- the name ----
    //
    // The one thing on the row that grows, with `flex_basis: 0` beside it:
    // grow alone divides only the slack left after every fixed column, which
    // on a narrow sheet is nothing at all.
    let name = dialog_text(
        commands,
        fonts,
        &row.name,
        TRAY_NAME_SIZE,
        palette::DIALOG_INK,
    );
    let name = clipped(
        commands,
        name,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            ..default()
        },
    );
    commands.entity(slot).add_child(name);

    // ---- the cost, drawn and not spelled ----
    //
    // The face is what carries it: a `BrowseRow` has the projected mana
    // *value*, which is what the sort key reads, and a number is not a price.
    let pips = spawn_cost(commands, fonts, row, view, faces);
    commands.entity(slot).add_child(pips);

    // ---- the type line ----
    let built = view.object(row.id).map(|o| faces.facts(o));
    let types = built
        .as_ref()
        .map_or_else(String::new, |f| f.type_line.clone());
    let type_line = dialog_text(
        commands,
        fonts,
        &types,
        TRAY_TYPE_SIZE,
        palette::DIALOG_SOFT,
    );
    let type_line = clipped(
        commands,
        type_line,
        Node {
            width: px(TRAY_TYPE_W),
            flex_shrink: 0.0,
            ..default()
        },
    );
    commands.entity(slot).add_child(type_line);

    let badge = spawn_badge(commands, lang, fonts, row);
    commands.entity(slot).add_child(badge);

    slot
}

/// What a grid needs that a row does not: the measure it shares out, and
/// whether the runs are headed.
///
/// A struct because [`spawn_grid`] would otherwise take eleven arguments, and
/// these four belong together — they are the answers to "how wide" and "how
/// many piles", which is the whole of what makes a grid different from a
/// list.
struct GridCtx<'a> {
    view: &'a PlayerView,
    statics: &'a GameStatic,
    fonts: &'a UiFonts,
    /// The width the tiles share, which is the sheet less the list's gutters.
    measure: f32,
    /// Whether to head each pile's run with its name.
    headed: bool,
}

/// Every card in the ticked zones, as tiles.
///
/// The owner's *"einfach nur alle Karten in der Zone wie auf einer
/// Produktseite"*. The rows are the same rows the lists draw — same filter,
/// same sort, same order — and the only thing that changes is that they are
/// laid across and wrapped instead of down.
///
/// The runs keep the sort's order rather than being regrouped by zone, which
/// is not a choice: [`BrowseZone`]'s `Ord` **is** the tab order and
/// `Browser::rows` already emits zone by zone in it, so walking the rows in
/// order and starting a new run whenever the zone changes gives exactly the
/// grouping the tabs promise. A grid that sorted itself again would be a
/// second opinion about an order that already has one.
#[allow(clippy::too_many_arguments)] // the rows, the measure, and the stores
fn spawn_grid(
    commands: &mut Commands,
    list: Entity,
    lang: Lang,
    rows: &[BrowseRow],
    ctx: GridCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
) {
    // Packed as whole tiles and drawn as pictures: what is shared out is the
    // node the wrap sees, and [`TRAY_TILE_CHROME`] is the difference.
    let (_, tile, air) = grid_across(
        ctx.measure,
        TRAY_BIG_THUMB_W + TRAY_TILE_CHROME,
        TRAY_TILE_MAX + TRAY_TILE_CHROME,
        TRAY_TILE_GAP,
    );
    let art = tile - TRAY_TILE_CHROME;
    let mut run: Option<(BrowseZone, Entity)> = None;
    for row in rows {
        let open = match run {
            Some((zone, node)) if zone == row.zone => node,
            _ => {
                if ctx.headed {
                    let head =
                        spawn_run_head(commands, lang, ctx.fonts, row.zone, ctx.view, ctx.statics);
                    commands.entity(list).add_child(head);
                }
                let node = commands
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: px(air),
                            row_gap: px(TRAY_TILE_GAP),
                            padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_TILE_GAP)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(list).add_child(node);
                run = Some((row.zone, node));
                node
            }
        };
        let node = spawn_tile(commands, lang, row, &ctx, textures, assets, cards, art);
        commands.entity(open).add_child(node);
    }
}

/// The line that starts one pile's run of tiles.
///
/// The pile's name and then a rule to the right edge, which is the cheapest
/// thing that reads as a heading in a panel whose only other horizontal line
/// is the rule under a row. It is drawn only when more than one pile is
/// showing: with one tab ticked the tabs above have already said which, and a
/// heading repeating the tab would be chrome on every grid the panel draws.
fn spawn_run_head(
    commands: &mut Commands,
    lang: Lang,
    fonts: &UiFonts,
    zone: BrowseZone,
    view: &PlayerView,
    statics: &GameStatic,
) -> Entity {
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                padding: UiRect::new(px(TRAY_SIDE), px(TRAY_SIDE), px(TRAY_TILE_GAP), px(0)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = dialog_label(
        commands,
        fonts,
        &zone_label(lang, zone, view, statics),
        10.5,
        palette::DIALOG_SOFT,
    );
    let rule = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                height: px(1),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_children(&[words, rule]);
    head
}

/// One card in the grid.
///
/// Nothing is written on it and nothing under it. A name under a 73-pixel
/// column clips on most of the pool, and a clipped name is worse than no name
/// — the hover preview already says which card this is, in full, with its
/// text. What the tile has to carry instead is the two things a picture
/// cannot say for itself: whether it is chosen, and where the keyboard is.
///
/// Those two are the same colour at two geometries, which is deliberate.
/// Chosen is the tile's **own** border going candle, tight against the art;
/// focus is a rail standing outside it. So a tile that is both reads as two
/// concentric rings and neither can be mistaken for the other — where a wash
/// over the art, which is what the list uses, would dim the one thing the
/// tile is *for*.
///
/// The ordering number is the list's own candle disc, moved into the tile's
/// top-left corner over the art. Carrying the same mark between the views is
/// what lets a player change view in the middle of an ordering without having
/// to learn it again.
#[allow(clippy::too_many_arguments)] // the row, the context, and the stores
fn spawn_tile(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    ctx: &GridCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
    width: f32,
) -> Entity {
    // The focus rail is a border the tile always reserves and only the
    // focused one paints, for the reason a row reserves its own: a ring that
    // appeared would move the tile, and a grid that stepped about as the
    // focus crossed it is worse to read than a grid with no focus at all.
    let tile = commands
        .spawn((
            TrayCard { object: row.id },
            Button,
            Node {
                width: px(width + TRAY_TILE_CHROME),
                padding: UiRect::all(px(2.0)),
                border: UiRect::all(px(TRAY_FOCUS)),
                // The card's own corner plus what stands outside it, so the
                // focus ring is concentric with the picture rather than
                // squarer than it — the same arithmetic the panel's head
                // does against the sheet.
                border_radius: BorderRadius::all(px(width * 0.0476 + TRAY_TILE_CHROME / 2.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            // Nothing, chosen or not — and that is the one place this tile
            // parts company with a row. A row says chosen with a wash across
            // its whole line; a tile's whole line *is* the picture, so the
            // wash would dim the one thing it is for. The candle edge below
            // carries the claim instead. Written as the plain rest colour
            // because `Feel` owns this field from its first tick: a wash set
            // here and not named in the `Feel` is a wash nobody ever sees.
            BackgroundColor(Color::NONE),
            BorderColor::all(if row.standing.focused {
                palette::CANDLE_EDGE
            } else {
                Color::NONE
            }),
            // Both ends stated, because `Feel::new` shades towards white and
            // keeps the alpha: a tile resting at nothing would be lifted to a
            // brighter nothing and never answer the pointer.
            //
            // Lifted, where a row is not. A row is a line of writing and
            // growing it grows the sentence; a tile is a picture, and the
            // pointer picking it up a little is what a picture answers with.
            // The gap is what it grows into — a `UiTransform` scale moves
            // nothing else on the row it is in.
            Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
        ))
        .id();

    let art = spawn_thumb(
        commands,
        lang,
        row,
        ctx.statics,
        textures,
        assets,
        ctx.fonts,
        cards,
        width,
    );
    // Chosen is the art's own edge, which is why it is inserted here rather
    // than being a property of the tile: a second frame around the picture
    // would read as a card in a holder.
    if row.standing.selected {
        commands
            .entity(art)
            .insert(Outline::new(px(2), px(0), palette::CANDLE));
    }
    commands.entity(tile).add_child(art);

    // The ordering number, over the art's top-left corner. Nothing is drawn
    // for a plain tick: the candle edge above has already said it, and a box
    // in the corner of a picture is a box over a picture.
    if row.place.is_some() {
        let disc = spawn_mark(commands, ctx.fonts, row, 20.0);
        commands.entity(disc).insert(Node {
            position_type: PositionType::Absolute,
            left: px(4),
            top: px(4),
            width: px(20),
            height: px(20),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        });
        commands.entity(tile).add_child(disc);
    }
    tile
}

/// The card's price, drawn and not spelled.
///
/// The face is what carries it: a [`BrowseRow`] has the projected mana
/// *value*, which is what the sort key reads, and a number is not a price.
fn spawn_cost(
    commands: &mut Commands,
    fonts: &UiFonts,
    row: &BrowseRow,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
) -> Entity {
    let pips = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(2),
                flex_shrink: 0.0,
                // A reserve rather than a fit, so the type lines beside them
                // start in one column down the whole list. A cost longer than
                // four pips takes the room it needs and pushes the name in,
                // which is the right way round: the name has the slack.
                min_width: px(TRAY_COST_W),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let built = view.object(row.id).map(|o| faces.facts(o));
    for symbol in built.iter().flat_map(|face| face.cost.iter()) {
        let pip = crate::manaui::spawn_pip(
            commands,
            fonts,
            baylee_client_core::manapip::pip(*symbol),
            TRAY_PIP,
        );
        commands.entity(pips).add_child(pip);
    }
    pips
}

/// Which pile the card is in, as a bordered word.
///
/// The bare zone word, with no seat on it: at a table of four the tabs above
/// already say whose pile is being looked through, and a seat name in a badge
/// this size is a smear. A token says so here instead — a graveyard holds
/// cards and tokens together and they are not the same thing, since a token
/// ceases to exist the next time state-based actions are checked (CR 111.7),
/// so a row that looked like a card would invite a player to plan around
/// something already gone.
///
/// Both lists draw it, and the large one draws it on a line of its own: a
/// merged list is the whole reason the badge exists, and dropping it there
/// would leave two Kommandozonen ticked and nothing on a row saying which one
/// a card came out of.
fn spawn_badge(commands: &mut Commands, lang: Lang, fonts: &UiFonts, row: &BrowseRow) -> Entity {
    let words = if row.token {
        Phrase::IsToken.text(lang).to_string()
    } else {
        row.zone.label().text(lang).to_string()
    };
    let text = dialog_text(
        commands,
        fonts,
        &words,
        TRAY_BADGE_SIZE,
        palette::DIALOG_SOFT,
    );
    let badge = commands
        .spawn((
            Node {
                width: px(TRAY_BADGE_W),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(5)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(text);
    badge
}

/// One row of the **large** list: the box, a picture at the one size the art
/// actually has, and beside them the name over the pile it came from.
///
/// It is the detailed row with the type line taken out and the picture
/// doubled, and the type line is the right thing to lose: it was the widest
/// fixed column on the row, and at 73 pixels the frame's colour and a
/// creature's silhouette are legible off the art itself. The name goes up one
/// size and the cost stays beside it, because a name and a price are what a
/// player is scanning for; the badge takes the second line, which is where the
/// room the type line gave up goes.
///
/// The box grows with the row rather than staying at 15: this is the view a
/// player picks when they are *choosing*, and a tick target that shrank
/// relative to everything around it would be the control going the wrong way.
#[allow(clippy::too_many_arguments)] // the row, the view, and the stores
fn spawn_big_row(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: &mut Option<&mut UiCards<'_>>,
) -> Entity {
    let slot = row_slot(commands, row, TRAY_BIG_ROW_H, TRAY_BIG_ROW_PAD);

    let mark = spawn_mark(commands, fonts, row, TRAY_BIG_BOX);
    commands.entity(slot).add_child(mark);

    let thumb = spawn_thumb(
        commands,
        lang,
        row,
        statics,
        textures,
        assets,
        fonts,
        cards,
        TRAY_BIG_THUMB_W,
    );
    commands.entity(slot).add_child(thumb);

    // The two lines beside the picture. The column grows and its children are
    // the ones allowed to clip, so a long name shortens instead of pushing
    // the cost off the row.
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(6),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let title = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                width: percent(100),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = dialog_text(
        commands,
        fonts,
        &row.name,
        TRAY_NAME_SIZE + 1.5,
        palette::DIALOG_INK,
    );
    let name = clipped(
        commands,
        name,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            ..default()
        },
    );
    let pips = spawn_cost(commands, fonts, row, view, faces);
    commands.entity(title).add_children(&[name, pips]);

    let badge = spawn_badge(commands, lang, fonts, row);
    commands.entity(words).add_children(&[title, badge]);
    commands.entity(slot).add_child(words);

    slot
}

/// What a zone tab reads. A pile belonging to a seat says whose it is,
/// because at a table of four "Graveyard" alone names nothing — and every tab
/// says how many cards are in it, which is the one thing the deleted pile
/// chips carried that nothing else on the sheet does.
///
/// The count goes in brackets rather than after a separator because the
/// dialog's typography already means something by a bracket: [`dialog_text`]
/// hands a bracketed run to [`palette::DIALOG_SOFT`], so "Graveyard (12)" is
/// drawn as a name with a grey aside beside it and reads as one.
fn zone_label(lang: Lang, zone: BrowseZone, view: &PlayerView, statics: &GameStatic) -> String {
    let name = zone.label().text(lang).to_string();
    let named = match zone.seat() {
        None => name,
        Some(seat) if seat == view.seat => name,
        Some(seat) => {
            let who = seat_name(lang, Some(statics), seat);
            Phrase::BrowseZoneOf.fill(lang, &[&name, &who])
        }
    };
    Phrase::BrowseTabCount.fill(lang, &[&named, &zone.count_in(view).to_string()])
}

/// The row itself: the thing a click lands on, before anything is written in
/// it.
///
/// Both lists build one of these, which is what keeps them one list in two
/// sizes rather than two lists — the picking, the chosen wash, the focus rail
/// and the hover behaviour are stated once here and neither view may differ
/// about them.
///
/// Candle, and a wash of it rather than a fill: a chosen row is still a row
/// being read. The tick and the ink carry the claim.
///
/// The hot end is stated both ways round, because `Feel`'s own hover keeps a
/// colour's alpha (see [`Feel::hot`]): an unchosen row rests at nothing and
/// would be lifted to a brighter nothing, and a chosen one rests at a tenth
/// and would be lifted to a paler tenth. A hundred rows that did not answer
/// the pointer is the whole list not answering it.
fn row_slot(commands: &mut Commands, row: &BrowseRow, height: f32, pad: f32) -> Entity {
    let (fill, hot) = if row.standing.selected {
        (palette::CANDLE_WASH, palette::CANDLE_WASH_LIT)
    } else {
        (Color::NONE, palette::DIALOG_LIT)
    };
    commands
        .spawn((
            TrayCard { object: row.id },
            Button,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                width: percent(100),
                // Explicit rather than whatever the thumbnail happens to make
                // it: the sheet's whole height is counted in rows, so a row
                // whose height was an accident of its tallest child would put
                // that arithmetic one text metric away from being wrong.
                height: px(height),
                padding: UiRect::axes(px(TRAY_SIDE), px(pad)),
                // Every row carries the focus rail's width, and only the
                // focused one carries its colour: a border that appeared
                // would shift that row's whole content sideways, and a list
                // whose rows step in and out as the focus passes is worse to
                // read than no focus at all.
                border: UiRect {
                    left: px(TRAY_FOCUS),
                    bottom: px(1),
                    ..default()
                },
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor {
                left: if row.standing.focused {
                    palette::CANDLE_EDGE
                } else {
                    Color::NONE
                },
                bottom: palette::DIALOG_LINE,
                ..BorderColor::all(Color::NONE)
            },
            // Lit, never lifted. A row is a *line of writing* — a tick, a
            // picture, a name, the pips, a type line — and `Feel::lift`'s own
            // doc names that as the case that must stay at zero: growing a
            // row grows the sentence on it. A hundred of them under a moving
            // pointer is a list that reflows while it is being read, which is
            // half of what the owner reported as the dialog flickering.
            Feel::tinting_to(fill, hot),
        ))
        .id()
}

/// The box a row is ticked in, or the number saying where it stands in an
/// ordering.
///
/// A number rather than a tick for an ordering, because "third" is not a
/// brighter kind of "chosen" — and it stands in the box's own place, so a
/// list of ordered cards reads down the same gutter a list of ticked ones
/// does. A row the question will not take leaves it empty rather than drawing
/// a box that cannot be ticked.
///
/// Four states decided before the node is spawned rather than patched into it
/// afterwards: a radius is a field of `Node` and not a component of its own,
/// so "insert a rounder corner" would mean writing the whole `Node` back over
/// itself.
///
/// `size` is the one thing the three views differ about, and the glyph inside
/// is scaled from it rather than given: a mark that kept a nine-point tick
/// while its box grew to twenty would be a tick rattling around in a box.
fn spawn_mark(commands: &mut Commands, fonts: &UiFonts, row: &BrowseRow, size: f32) -> Entity {
    let (fill, edge, radius, glyph_in) = if row.place.is_some() {
        (
            palette::CANDLE,
            palette::CANDLE,
            size / 2.0,
            row.place.map(|place| place.to_string()),
        )
    } else if row.standing.selected {
        (
            palette::CANDLE,
            palette::CANDLE,
            3.0,
            Some(glyph::CHECK.to_string()),
        )
    } else if row.standing.selectable {
        (Color::NONE, palette::DIALOG_SOFT, 3.0, None)
    } else {
        (Color::NONE, Color::NONE, 3.0, None)
    };
    let mark = commands
        .spawn((
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(radius)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(inside) = glyph_in {
        // The ordering's number is set in the text face and the tick in the
        // icon one, because a tick is a glyph the text face does not have.
        // Checked again on the change from Inter: Alegreya Sans has no
        // U+2713 either.
        let face = if row.place.is_some() {
            tf(fonts, size * 9.5 / TRAY_BOX)
        } else {
            icon_tf(fonts, size * 9.0 / TRAY_BOX)
        };
        let ink = commands
            .spawn((
                Text::new(inside),
                face,
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(mark).add_child(ink);
    }
    mark
}

/// The card's picture, at whatever size the view draws it.
///
/// `built` is deliberately not passed to it: a built face draws the name, the
/// cost and the type line onto the card, and at forty pixels wide all three
/// would be a grey smear. A list says those three things beside it, in
/// letters a person can read; a grid says them in the hover preview.
///
/// Every view asks for the same [`ImageKey`] — the one the row already
/// carries — so making the picture bigger costs no texture at all. See
/// [`TRAY_BIG_THUMB_W`] for what bounds the size.
///
/// [`ImageKey`]: baylee_client_core::images::ImageKey
#[allow(clippy::too_many_arguments)] // a row, the stores, and one number
fn spawn_thumb(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: &mut Option<&mut UiCards<'_>>,
    width: f32,
) -> Entity {
    let height = width * 88.0 / 63.0;
    let thumb = if let Some(key) = row.art {
        let image = textures.get(key, statics, assets);
        spawn_card_art(
            commands,
            lang,
            image,
            None,
            width,
            height,
            crate::face::Detail::Compact,
            fonts,
            // No keyword sheath: nothing in the browser is on a battlefield,
            // and a card in a graveyard wearing an indestructible border
            // would be claiming something the rules do not say.
            CardLook::art(key, finish_of(statics, Some(key)), 0),
            cards.as_deref_mut(),
        )
    } else {
        // A token in a graveyard, or a card this seat may not identify.
        commands
            .spawn((
                Node {
                    width: px(width),
                    height: px(height),
                    flex_shrink: 0.0,
                    border_radius: card_radius(width),
                    ..default()
                },
                BackgroundColor(palette::DIALOG_LIT),
            ))
            .id()
    };
    commands.entity(thumb).insert(Pickable::IGNORE);
    thumb
}

#[cfg(test)]
mod tests;
