//! The ability sheet: what a permanent can do, on parchment, beside the card.
//!
//! It replaces a row of buttons in the prompt bar. Those buttons carried the
//! ability's *cost* as their whole label — `{2}, {T}` — because a button on a
//! bar has room for four words, so a player read what an ability charged and
//! never what it did, and the bar was at the bottom of the window while the
//! permanent was on the table.
//!
//! What is here is a sheet of paper laid beside the card it belongs to: the
//! permanent's name, and a numbered row per thing it can do with the
//! ability's own printed sentence on it. The cost is under the key that arms
//! it, in a narrow column down the left — the two marks a player scans rather
//! than reads, with the whole of the rest of the row left to the sentence.
//! `docs/redesign-proposal.md` §7 is the design.
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
use baylee_client_core::abilitysheet;
use baylee_client_core::card_face::TextBlock;

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
const SHEET_MAX: f32 = 380.0;

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

/// How much light the paper has lost where the nub lies on it.
///
/// The nub carries the sheet's own grain, which is what makes it the same
/// material — and stretching a 512-pixel sheet of parchment into fourteen
/// pixels shows the *whole* of it, the bright middle included, while the
/// sheet under it at that point is showing its own vignetted rim. Measured on
/// the running client: the paper beside the nub is 203,187,148 and the nub's
/// face came out 215,201,163. This is the rim's share of the middle's light,
/// which is the one number that closes it — and it is a tint on the image and
/// not a second colour, so the grain still shows through.
const NUB_TONE: f32 = 0.945;

/// How far the halo stands off the card it rings.
const HALO_AIR: f32 = 3.0;

/// How long the sheet takes to open, in seconds.
///
/// Short enough to be over before a player has finished looking down at it —
/// the sheet answers a click, and an answer that takes a quarter of a second
/// to arrive is a delay rather than a movement.
const ZOOM_IN: f32 = 0.16;

/// And to close.
///
/// Shorter still, because the two are not the same event: opening is an
/// answer arriving and closing is it being dismissed, and a dismissal that
/// took as long as the answer reads as reluctance.
const ZOOM_OUT: f32 = 0.10;

/// The size the sheet grows from, and shrinks back to.
///
/// Not zero. A sheet that grows out of nothing is a puff of smoke; this is a
/// page being laid down — it was always this size, and the movement is the
/// last eighth of it arriving.
const ZOOM_FROM: f32 = 0.88;

/// The overshoot's shape, as the `c₁` of the usual ease-out-back.
///
/// The curve runs 0 → 1 over the range [`ZOOM_FROM`]..1 and its peak is
/// `4c³ / 27(c+1)²` of that range past the end. **Three** is the value where
/// that closed form collapses to exactly a quarter — `4·27 / 27·16` — so the
/// sheet overshoots by a clean `(1 − ZOOM_FROM)/4`, which is 3% of full size.
///
/// The textbook `1.70158` is for a curve whose range *is* the whole size; at
/// this range it would overshoot by a third of a per cent and there would be
/// no snap at all.
const ZOOM_BACK: f32 = 3.0;

/// The share of the opening the nub and the halo wait out.
///
/// They are attached to things — the sheet's edge and the card's border — and
/// a sheet at 90% has its edge 5% of its height away from where the nub is
/// drawn. Rather than animate that gap away, the two arrive once the paper is
/// nearly full size, which also reads right: the sheet opens, and *then* it
/// points.
const ZOOM_TAIL: f32 = 0.55;

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

/// A keycap's side, as a multiple of the legend on it.
///
/// A square and not a disc, because what it stands for is a **key**: the digit
/// on it is the one a player presses to arm that row, and a keyboard has no
/// round keys. It was a roundel and read as a bullet — an ornament numbering
/// a list rather than a control naming a keystroke.
///
/// A *ratio* and not a side, because the sheet draws caps at two sizes. It was
/// 21 px flat, which is 1.9 times the rows' 11 pt and was right there and
/// wrong everywhere else: the footer's smaller legend sat in the same 21 px
/// box, so the quietest key on the sheet had the largest cap on it.
const KEYCAP_SIDE: f32 = 1.9;

/// The keycap's corner radius, and the row's.
///
/// One constant for both, so the cap reads as a key sitting *in* its row
/// rather than as a second, differently-cornered object on it. A key is a
/// square with its corners taken off, which is what a small radius on a
/// 21-pixel square is; half the side is the circle it used to be.
const KEYCAP_R: f32 = 4.0;

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

/// How much of [`palette::PARCHMENT_EDGE`] a title's hairline is drawn in.
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

/// The dish the close button sits in — [`palette::PARCHMENT_INK`] at 5%.
///
/// A resting state at all, which the other controls on this sheet do without,
/// because a **finger** has no hover: on a tablet the cross would otherwise be
/// a glyph floating on paper with nothing to say it is a target. Five per cent
/// is a press in the paper rather than a button on it.
const CLOSE_REST: Color = Color::srgba(0.098, 0.082, 0.062, 0.05);

/// The same with the pointer on it.
const CLOSE_HOT: Color = Color::srgba(0.098, 0.082, 0.062, 0.14);

/// The wash under the row the keyboard is on.
///
/// The same claim at half the weight, because the two are different claims
/// about the same row: the cursor is *where a key would land* and the arming
/// is *what a key has already done*.
const PICKED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.08);

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
    let ink = palette::PARCHMENT_INK.to_srgba();
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
    /// Where the three pieces already are, so a camera standing still costs
    /// one comparison instead of a relayout of the whole sheet.
    placed: Option<Placement>,
}

/// Where the sheet, its nub and its halo were last put.
///
/// The sheet's own corner is **not** enough to guard on any more. It is
/// clamped to the window, so a card gliding along the bottom row moves the
/// halo and the nub while leaving the corner exactly where it was — and a
/// guard that only watched the corner would pin the halo to where the card
/// used to be. What changed is that the sheet stopped being the only thing
/// this system places.
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
    let ink = palette::PARCHMENT_EDGE;
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

/// Puts the halo, a hairline standing off the card's own box.
fn put_halo(at: &Placement, node: &mut Node) {
    node.display = Display::Flex;
    node.left = px(at.mid.x - at.card.x / 2.0 - HALO_AIR);
    node.top = px(at.mid.y - at.card.y / 2.0 - HALO_AIR);
    node.width = px(at.card.x + 2.0 * HALO_AIR);
    node.height = px(at.card.y + 2.0 * HALO_AIR);
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
#[derive(Component)]
pub struct SheetNub;

/// The hairline round the card the sheet belongs to.
///
/// Made of the **sheet** and not of the card: [`palette::PARCHMENT_EDGE`] at
/// the sheet's own weight and corner radius, so the paper, its tail and this
/// ring are one gesture in one material. The card's border already carries
/// three lights that are rules claims — can be activated, is armed, will be
/// tapped — and "this paper is about this card" is not a fourth one; a light
/// in that register would be read as something the engine had said.
#[derive(Component)]
pub struct SheetHalo;

/// The two pieces that are attached to something: the tail and the ring.
///
/// `Without<AbilitySheet>` is not decoration — it is what lets a system hold
/// a mutable `Node` (or `UiTransform`) on the sheet and on these at once.
type Trim = (Or<(With<SheetNub>, With<SheetHalo>)>, Without<AbilitySheet>);

/// The ring on its own, disjoint from both of the others.
type Halo = (With<SheetHalo>, Without<AbilitySheet>, Without<SheetNub>);

/// And the tail on its own.
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

/// What the sheet is currently saying, so it is rebuilt only when that
/// changes.
///
/// The options themselves are fingerprinted by their labels rather than
/// stored: the list is rebuilt from `LegalActions` on every read anyway, and
/// what the sheet draws is exactly what those labels say.
#[derive(Resource, Default)]
pub struct SheetRevision {
    object: Option<ObjectId>,
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
    /// label for as long as it stood.
    texts: usize,
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
    let crate::Deed::Ability(action) = &armed.deed else {
        return None;
    };
    options.iter().position(|o| &o.action == action)
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
    let open = duel
        .ability_menu
        .and_then(|object| Some((object, ability_options(&duel, lang, object)?)))
        .filter(|(_, options)| options.len() > 1);

    let Some((object, options)) = open else {
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

    let page = abilitysheet::clamp(options.len(), duel.ability_page);
    let armed = armed_row(&duel, object, &options);
    let fingerprint: Vec<String> = options.iter().map(|o| o.label.clone()).collect();
    if revision.object == Some(object)
        && revision.page == page
        && revision.pick == duel.ability_pick
        && revision.armed == armed
        && revision.lang == Some(lang)
        && revision.fingerprint == fingerprint
        && revision.texts == faces.len()
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
    revision.page = page;
    revision.pick = duel.ability_pick;
    revision.armed = armed;
    revision.lang = Some(lang);
    revision.fingerprint = fingerprint;
    revision.texts = faces.len();

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
            // does not mean what the four in `hud::Z_SLIP` means. bevy's
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
        &options,
        page,
        armed,
        fresh,
        standing,
    );
    let (halo, nub) = spawn_trim(&mut commands, &sheets, fresh, standing);
    // Order *is* the drawing: the ring round the card, then the paper, then
    // the tail lying across the paper's edge. See [`SheetNub`] for why the
    // tail is in front of the sheet and not behind it.
    commands.entity(root).add_children(&[halo, sheet, nub]);
}

/// The ring round the card and the tail that points at it.
///
/// Both are spawned hidden unless the sheet they belong to is being *rebuilt*
/// and `standing` says where the last one stood. [`place_ability_sheet`] is
/// otherwise what reveals them, and it needs a `ComputedNode` that does not
/// exist on the frame they are made — so without this they would be drawn
/// once in the window's top-left corner, which is where an unplaced absolute
/// node is.
fn spawn_trim(
    commands: &mut Commands,
    sheets: &UiSheets,
    fresh: bool,
    standing: Option<Placement>,
) -> (Entity, Entity) {
    let arrive = Vec2::splat(if fresh { 0.0 } else { 1.0 });
    let mut halo_node = Node {
        position_type: PositionType::Absolute,
        display: Display::None,
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(4)),
        ..default()
    };
    if let Some(at) = standing.as_ref() {
        put_halo(at, &mut halo_node);
    }
    let halo = commands
        .spawn((
            SheetHalo,
            halo_node,
            BorderColor::all(palette::PARCHMENT_EDGE),
            UiTransform::from_scale(arrive),
            // The ring is over the card it rings. A ring that answered the
            // pointer would be a card that stopped answering it.
            Pickable::IGNORE,
        ))
        .id();
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
            // No ground of its own. The grain below covers the *padding* box,
            // and a `BackgroundColor` reaches under the border as well — so a
            // flat [`palette::PARCHMENT`] here survived the grain as a
            // one-pixel bright rim on the two edges the placer leaves clear,
            // which at a forty-five degree angle is an antialiased light line
            // down each side of the diamond. Measured: 213 against the
            // paper's 203, and the last thing drawing an outline round a tail
            // that is meant to have none.
            BackgroundColor(Color::NONE),
            // Written by the placer, which is the only thing that knows which
            // two of the four edges are the ones facing out — or carried
            // straight over from the sheet this one replaces.
            nub_edge,
        ))
        .id();
    // **The nub is cut from the same paper, and this is what says so.** It
    // was a flat [`palette::PARCHMENT`] diamond — 224,212,176 — lying on a
    // sheet whose face is the *stretched grain*, measured at 203,187,148. Its
    // inner half therefore painted a bright flat patch on grained paper and
    // the outer half a bright flat tail, which is the whole reason it read as
    // its own element rather than as the sheet's corner: a tail of paper and
    // the paper it is torn from cannot be two materials. The geometry was
    // right all along — only `NUB·√2/2` of it stands proud — so nothing here
    // moves it or grows it.
    //
    // An absolute child inset to zero rather than [`sheet_surface`], which
    // carries the sheet's own 13-pixel corner and would round a 14-pixel
    // square into a circle, and rather than [`sheet`] on the nub itself,
    // which paints the *content* box and would leave the border ring flat.
    // The rotation and the opening scale propagate to it; the grain is noise
    // and has no up.
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
            ImageNode {
                color: Color::srgb(NUB_TONE, NUB_TONE, NUB_TONE),
                ..sheet(sheets)
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(nub).add_child(grain);
    (halo, nub)
}

/// One sheet, built.
#[allow(clippy::too_many_arguments)]
fn spawn_sheet(
    commands: &mut Commands,
    fonts: &UiFonts,
    sheets: &UiSheets,
    faces: &crate::cardtext::CardTexts,
    lang: Lang,
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
    page: usize,
    armed: Option<usize>,
    fresh: bool,
    standing: Option<Placement>,
) -> Entity {
    let mut node = Node {
        position_type: PositionType::Absolute,
        // As wide as what is on it. An absolutely-positioned node
        // shrink-wraps its content, so the bound is the pair of limits and
        // not a width — and the rows have to be able to *ask* for their
        // natural width for that to mean anything, which is what
        // `flex_basis: Auto` on a row's prose column is for.
        width: Val::Auto,
        min_width: px(SHEET_MIN),
        max_width: px(SHEET_MAX),
        flex_direction: FlexDirection::Column,
        padding: UiRect::vertical(px(SHEET_PAD_Y)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    };
    if let Some(at) = standing.as_ref() {
        put_sheet(at, &mut node);
    }
    let sheet = commands
        .spawn((
            AbilitySheet {
                object,
                // Left unknown even where the sheet was put back at
                // `standing`, so [`place_ability_sheet`] runs its whole body
                // once more against the card as it stands now. Putting it
                // back is about not *blinking*; it is not a claim that the
                // card has not moved since.
                placed: None,
            },
            // `fresh` is what keeps this a movement and not a twitch: the
            // sheet is rebuilt whenever a row is armed, the cursor moves or
            // the page turns, and a sheet that replayed its opening on every
            // keystroke would be unreadable. Only a sheet that is about a
            // *different* permanent than the last one opens.
            SheetZoom {
                t: if fresh { 0.0 } else { 1.0 },
                closing: false,
            },
            UiTransform::from_scale(Vec2::splat(if fresh { ZOOM_FROM } else { 1.0 })),
            // **This is the flicker.** A rebuilt tree has no `ComputedNode`
            // until `bevy_ui` has laid it out — so for one frame
            // [`place_ability_sheet`] centred a sheet of size zero, which put
            // it half a sheet to the right of where it belongs.
            //
            // Hidden and not `Display::None`, which would be the obvious
            // thing and is the wrong one: a `display: none` node is not laid
            // out at all, so it would never acquire the size it is waiting
            // for. Visibility is a render concern and the layout runs anyway.
            //
            // A *rebuild* is spared the hidden frame entirely, because
            // `standing` already says where the sheet was and a rebuild moves
            // nothing. Without that the sheet blinked out and back once per
            // click, which is what the owner saw.
            if standing.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            node,
            BackgroundColor(palette::PARCHMENT),
            BorderColor::all(palette::PARCHMENT_EDGE),
            // The house's own, which stands the sheet further off the table
            // than the shallower one written out here did: this is a piece of
            // paper lying *over* the board, not a panel in the same plane as
            // one, and it is the same claim the tray and the finish card
            // make. One helper rather than a fourth set of four numbers.
            crate::hud::sheet_shadow(),
            // Deliberately *not* `Pickable::IGNORE`. The root is, and the
            // grain is, so the felt around the sheet still belongs to the
            // table — but a click on the paper itself has to land on the
            // paper. Falling through would reach the card underneath and
            // re-run `activate_card`, which puts the page and the cursor
            // back to the top of the list a player was reading.
        ))
        .id();
    commands.spawn(sheet_surface(sheets)).insert(ChildOf(sheet));

    spawn_head(commands, fonts, faces, duel, object, sheet);

    // ---- the rows --------------------------------------------------------
    //
    // Whether *this sheet* has a cost column, and how wide, is one answer for
    // all of it and not one per row: the sentences share a left edge, and a
    // row with a column of its own width would put its prose where its
    // neighbours' costs are. Decided over the rows on this page, because a
    // page is what is seen.
    let costs = cost_column(faces, duel, object, options, page);
    for at in abilitysheet::rows(options.len(), page) {
        let digit = digit_of(at).unwrap_or('?');
        let row = spawn_row(
            commands,
            fonts,
            faces,
            duel,
            object,
            at,
            &options[at],
            digit,
            armed == Some(at),
            duel.ability_pick == at,
            costs,
        );
        commands.entity(sheet).add_child(row);
    }
    if abilitysheet::paged(options.len()) {
        let row = spawn_pager(
            commands,
            fonts,
            lang,
            page,
            abilitysheet::pages(options.len()),
        );
        commands.entity(sheet).add_child(row);
    }

    spawn_foot(commands, fonts, lang, armed, sheet);
    sheet
}

/// The permanent's name, the way out, and the hairline under them.
///
/// There used to be a line of spaced capitals between the two saying what the
/// list below was ("what it can do"). It was the one place in the interface
/// with letter-spacing, and it is gone: a list of things a permanent can do,
/// standing under that permanent's own name, does not need a label saying it
/// is a list of things a permanent can do. The hairline does the separating
/// the label was also doing, and it is what remains.
fn spawn_head(
    commands: &mut Commands,
    fonts: &UiFonts,
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    object: ObjectId,
    sheet: Entity,
) {
    /// The close button's side.
    ///
    /// Half again the widest keycap and close to the 44 logical pixels the
    /// lobby gives a phone — the sheet is not a responsive screen, it is
    /// pinned to a card 47 px wide, so a target sized for a thumb would be a
    /// sixth of the paper. This is sized for a finger on a tablet, which is
    /// what a card on a table is played with, and it is the only thing on
    /// the sheet a pointer has to *find* rather than being handed by a row.
    const CLOSE: f32 = 30.0;

    let name = duel.view.as_ref().map_or_else(String::new, |view| {
        view.object(object)
            .map_or_else(String::new, |o| crate::face::name_of(o, view, faces))
    });
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                margin: UiRect::horizontal(px(SHEET_PAD_X)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Text::new(name),
            // Bold and in the house colour. The name is the one thing on this
            // sheet that is not an answer to anything — it says *whose* list
            // this is — so giving it the accent costs the accent nothing,
            // while leaving it in the body ink made the head read as a first
            // row.
            //
            // Brass and not `palette::ACCENT`: the teal belongs to the panel
            // register, and `docs/redesign-proposal.md` §1.3 is what splits
            // the two. On parchment the accent *is* brass — it is what the
            // armed keycap and the armed wash are already made of. At ink
            // weight, though: `BRASS` itself is a light and measures 1.6:1 as
            // letters here, which `the_parchment_writes_no_letters_in_brass`
            // holds.
            tf_bold(fonts, 16.0),
            TextColor(palette::INK_BRASS),
            Node {
                // It takes the slack, so the cross is against the right
                // margin whatever the name is and however wide the sheet
                // came out — and `min_width: 0` lets a long name wrap rather
                // than push the cross off the paper.
                flex_grow: 1.0,
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(head);

    let shut = commands
        .spawn((
            SheetClose,
            Node {
                width: px(CLOSE),
                height: px(CLOSE),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(CLOSE_REST),
            // The paper taking a press, rather than the brass every other
            // wash on this sheet is made of: the rows are lit in the colour
            // of committing to something and this is the way *out*. Reaching
            // for the arming colour here would say the door was a deed.
            Feel::rising_to(CLOSE_REST, CLOSE_HOT),
        ))
        .id();
    let cross = commands
        .spawn((
            // `×` (U+00D7), which Alegreya Sans' Bold cut carries — the
            // dedicated multiplication and ballot crosses (U+2715, U+2716)
            // are not in the family and would draw as tofu.
            Text::new("\u{d7}"),
            tf_bold(fonts, 18.0),
            // The same grey as the way out in the footer. The two are one
            // door drawn twice, once for each hand.
            TextColor(palette::SLIP_ASIDE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shut).add_child(cross);
    commands.entity(row).add_child(shut);

    commands.entity(sheet).add_child(row);
    let hair = rule(commands, 5.0);
    commands.entity(sheet).add_child(hair);
}

/// The rule under the rows, and the one line that says which key does what.
///
/// Two halves pushed to opposite ends: what the digit on the armed row would
/// do if it were pressed again, and the way out. With nothing armed the first
/// half names the gesture instead, because a sheet that said only "Esc closes"
/// would be a list with no stated way to answer it.
///
/// The keys in it are **drawn as keys** (see [`cap`]) rather than spelled out
/// mid-sentence, which is how the settings screen has always shown a binding.
/// The legend for the way out comes from
/// [`baylee_client_core::prefs::Chord`] rather than from a phrase, so the word
/// on the cap here and the word on that screen cannot drift apart — and so a
/// translation cannot rename a key.
fn spawn_foot(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    armed: Option<usize>,
    sheet: Entity,
) {
    /// The footer's own size, a little under the rows'.
    const FOOT_PT: f32 = 10.5;
    /// The way out is drawn smaller than the digit that sends.
    ///
    /// The two halves are not a pair: one is the next thing a player is going
    /// to do and the other is the door, which is in the same place on every
    /// sheet and only has to be findable. Size and colour say that once each.
    const EXIT_PT: f32 = 9.5;
    /// The air between the hairline and the footer's own line.
    ///
    /// More than the 2 px [`rule`] leaves under itself, because the rule is
    /// parting two *kinds* of writing here and not two rows of the same kind:
    /// above it is what this permanent can do, below it is how the keyboard
    /// works.
    const FOOT_AIR: f32 = 6.0;

    let hair = rule(commands, 8.0);
    commands.entity(sheet).add_child(hair);
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                // Both halves on the exit's middle. `Center` and not
                // `SpaceBetween`'s default baseline: the two halves are
                // different heights (a cap plus words against words alone),
                // and a line of prose beside a keycap reads as sitting low
                // unless something says otherwise.
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(8),
                margin: UiRect::new(px(SHEET_PAD_X), px(SHEET_PAD_X), px(FOOT_AIR), px(0.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let words = |commands: &mut Commands, text: String, size: f32| {
        commands
            .spawn((
                Text::new(text),
                tf(fonts, size),
                TextColor(palette::SLIP_ASIDE),
                Pickable::IGNORE,
            ))
            .id()
    };

    // The left half: a cap only where there is a particular key to draw. With
    // nothing armed the sentence is about *any* digit, and a cap reading "1"
    // there would name the first row rather than the gesture.
    let left = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(digit) = armed.and_then(digit_of) {
        let key = cap(
            commands,
            fonts,
            &digit.to_string(),
            palette::BRASS,
            // Dark on gold. White on brass fails contrast, and an armed row
            // is the one a player is about to commit to.
            palette::PARCHMENT_INK,
            FOOT_PT,
        );
        commands.entity(left).add_child(key);
        let says = words(
            commands,
            Phrase::SheetPressAgain.text(lang).to_string(),
            FOOT_PT,
        );
        commands.entity(left).add_child(says);
    } else {
        let says = words(
            commands,
            Phrase::SheetDigitPicks.text(lang).to_string(),
            FOOT_PT,
        );
        commands.entity(left).add_child(says);
    }
    commands.entity(foot).add_child(left);

    let right = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let key = cap(
        commands,
        fonts,
        &baylee_client_core::prefs::Chord::key("Escape").display(),
        palette::SLIP_GHOST,
        // The same grey as the words beside it, so the cap and its sentence
        // are one aside instead of a black key with a quiet label.
        palette::SLIP_ASIDE,
        EXIT_PT,
    );
    commands.entity(right).add_child(key);
    let says = words(
        commands,
        Phrase::SheetCloses.text(lang).to_string(),
        EXIT_PT,
    );
    commands.entity(right).add_child(says);
    commands.entity(foot).add_child(right);

    commands.entity(sheet).add_child(foot);
}

/// The digit drawn on the row at `at`, wherever on its page that is.
fn digit_of(at: usize) -> Option<char> {
    u32::try_from(at % abilitysheet::PAGE + 1)
        .ok()
        .and_then(|place| char::from_digit(place, 10))
}

/// One key, drawn as the key it is.
///
/// The sheet names a key in two places — on a row, where the digit is what
/// arms it, and in the footer, where `Esc` is the way out — and a player
/// should not have to learn twice that a small square means "press this".
/// `lobby::ui::chip` is the same idea in the panel register, which is where
/// the settings screen draws every binding; this is the parchment one.
///
/// It **grows with its legend**: a digit is one character and `Esc` is three,
/// so the side is a floor and not a width. Anything else would either clip
/// the word or make every digit sit in a box wide enough for the longest key
/// on the keyboard. The box grows with the *size* too — see [`KEYCAP_SIDE`].
///
/// `ink` is the legend's colour and is not derived from `fill`, because the
/// two say different things. On a row the cap is a control and is written in
/// full ink; in the footer it is a reminder of a key that is always there, and
/// is written in the same grey as the words beside it.
fn cap(
    commands: &mut Commands,
    fonts: &UiFonts,
    legend: &str,
    fill: Color,
    ink: Color,
    size: f32,
) -> Entity {
    let side = size * KEYCAP_SIDE;
    let key = commands
        .spawn((
            Node {
                min_width: px(side),
                height: px(side),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(size * 0.45)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(if fill == palette::BRASS {
                palette::PARCHMENT_INK
            } else {
                palette::PARCHMENT_SOFT
            }),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(legend.to_string()),
            tf_bold(fonts, size),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(key).add_child(glyph);
    key
}

/// A hairline across the sheet, with `above` pixels of air over it.
///
/// Inset like the ink it separates rather than run wall to wall: a rule that
/// touched the border would close the sheet into two boxes, and what it is
/// doing is parting two kinds of writing on one page.
fn rule(commands: &mut Commands, above: f32) -> Entity {
    commands
        .spawn((
            Node {
                height: px(1),
                margin: UiRect::new(px(SHEET_PAD_X), px(SHEET_PAD_X), px(above), px(2)),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT_EDGE),
            Pickable::IGNORE,
        ))
        .id()
}

/// What a row charges, where that is not the row's own words a second time.
///
/// Offline there is no card text at all, so every printed ability falls back
/// to its cost as its *label* — and a row reading `{2}, {T}` beside `{2}, {T}`
/// is the duplication the sentence-first layout exists to remove, not a cost
/// drawn where a cost belongs.
///
/// Its own function because the answer is needed twice: once per row, to draw
/// it, and once per *sheet*, to decide whether there is a cost column at all.
fn row_cost(option: &crate::abilities::AbilityOption, printed: bool) -> Option<&str> {
    let repeats = !printed && option.cost.as_deref() == Some(option.label.as_str());
    option.cost.as_deref().filter(|_| !repeats)
}

/// How wide this sheet's **cost column** is, or `None` for no column at all.
///
/// One answer for the whole page, because a column is a claim about where
/// every sentence on the sheet begins — a row that opted out would start its
/// prose where its neighbours' costs are.
///
/// A column is a place for marks, and for *one line* of them. Two things take
/// a cost out of it, and both were seen on the paper before they were written
/// down here:
///
/// - **Words.** A card asks for a tap and a colour in marks and for everything
///   else in prose — `Sacrifice this artifact`, `Pay 1 life` — and three
///   marks' width is not a place for a sentence; it overhung the paper.
/// - **A second payment.** `{2}{U}{U}, {T}` is two lines in any column narrow
///   enough to be one, and a two-line cost beside a one-line sentence reads as
///   a row that has slipped, because the sentence is centred between them.
///   [`COST_MARKS`] caps the width instead, so a *single* payment too wide for
///   it wraps against a sentence long enough to stand beside it.
///
/// Either one and the page puts every cost where the printed card puts it:
/// [`spawn_cost_title`], a title line over the sentence it charges for, with
/// the row's whole width to wrap in.
fn cost_column(
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
    page: usize,
) -> Option<f32> {
    let mut widest: Option<f32> = None;
    for at in abilitysheet::rows(options.len(), page) {
        let printed = row_text(faces, duel, object, &options[at]).is_some();
        let Some(cost) = row_cost(&options[at], printed) else {
            continue;
        };
        let mut payments = crate::abilities::payments(cost);
        let Some(only) = payments.next() else {
            continue;
        };
        if payments.next().is_some() {
            return None;
        }
        let span = crate::manaui::marks_span(only, ROW_PT, COST_MARK)?;
        widest = Some(widest.map_or(span, |wide: f32| wide.max(span)));
    }
    widest.map(|wide| wide.min(COST_MARKS))
}

/// A cost as the narrow column left of the sentence.
///
/// `width` is [`cost_column`]'s answer — the widest cost on the page, capped —
/// and the column is spawned at it even for a row that has no cost, so every
/// sentence on the sheet begins in the same place.
fn spawn_cost_column(
    commands: &mut Commands,
    fonts: &UiFonts,
    cost: Option<&str>,
    width: f32,
) -> Entity {
    let purse = commands
        .spawn((
            Node {
                width: px(width),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(cost) = cost {
        let pips = crate::manaui::spawn_rich_marks(
            commands,
            fonts,
            cost,
            ROW_PT,
            COST_MARK,
            palette::SLIP_SOFT,
        );
        commands.entity(purse).add_child(pips);
    }
    purse
}

/// A cost as a **title over the sentence it charges for**, and a rule under it.
///
/// Where the printed card puts it — `{1}, {T}, Sacrifice this artifact: Draw
/// a card`, the charge first and what it buys after — and where a cost
/// [`cost_column`] will not take has to go.
///
/// The hairline is what tells the two apart at a glance, and it is drawn at
/// [`COST_RULE_WASH`] rather than at [`rule`]'s full strength because a cost
/// and its sentence are one thought. It is a child of the holder so that the
/// holder is the whole title — one thing to place, and one margin under it.
///
/// The line itself sits in a holder rather than having its own `Node`
/// patched: that node belongs to `manaui::rich`, and a second copy of it
/// written out here to add one margin is a copy that drifts from it.
fn spawn_cost_title(commands: &mut Commands, fonts: &UiFonts, cost: &str) -> Entity {
    let holder = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                margin: UiRect::bottom(px(COST_TITLE_AIR)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let line = crate::manaui::spawn_rich_marks(
        commands,
        fonts,
        cost,
        ROW_PT,
        COST_MARK,
        palette::SLIP_SOFT,
    );
    commands.entity(holder).add_child(line);
    let hair = commands
        .spawn((
            Node {
                height: px(1),
                margin: UiRect::top(px(COST_TITLE_AIR)),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT_EDGE.with_alpha(COST_RULE_WASH)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(holder).add_child(hair);
    holder
}

/// One row: what it costs, what the ability does, the key that arms it.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn spawn_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    object: ObjectId,
    index: usize,
    option: &crate::abilities::AbilityOption,
    digit: char,
    armed: bool,
    picked: bool,
    costs: Option<f32>,
) -> Entity {
    let wash = if armed {
        ARMED_WASH
    } else if picked {
        PICKED_WASH
    } else {
        Color::NONE
    };
    let row = commands
        .spawn((
            // The *whole row* is the button, not the keycap on it: a target
            // 21 pixels across beside a sentence that is not clickable is a
            // row a player aims at and misses.
            crate::hud::AbilityButton { index },
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                padding: UiRect::axes(px(SHEET_PAD_X), px(6)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(wash),
            // Lit but not lifted. A row is a *sentence* with a wash behind it,
            // and a button's 2.5% grow-on-hover reflows that sentence every
            // time the pointer crosses it — which the owner read as the text
            // changing size, because that is exactly what it is.
            Feel::tinting_to(wash, pressed(wash)),
        ))
        .id();

    let keycap = cap(
        commands,
        fonts,
        &digit.to_string(),
        if armed {
            palette::BRASS
        } else {
            palette::SLIP_GHOST
        },
        // Dark on gold in both states. White on brass fails contrast, and an
        // armed row is the one a player is about to commit to.
        palette::PARCHMENT_INK,
        ROW_CAP_PT,
    );
    // What the ability *does*, if the card's text is here to say it, and what
    // it costs if it is not: the fallback label is `printed_label`'s answer,
    // which is the cost. Read here rather than where it is drawn, because the
    // cost column is decided by it and stands to the left of the sentence.
    let printed = row_text(faces, duel, object, option);
    let cost = row_cost(option, printed.is_some());
    // **Cost, sentence, key**, in that order across the row. The cost is what
    // a player checks first ("can I afford this") and the key is what they
    // press last, so the row is read in the order it is used; and both ends
    // hold a fixed width, which leaves the sentence one straight left edge
    // down the whole sheet.
    //
    // The column is spawned on every row of such a sheet, empty or not. A row
    // that dropped it would start its sentence at the sheet's edge while its
    // neighbours started fifty pixels in, and a ragged column of prose is a
    // worse answer than a little empty paper.
    if let Some(width) = costs {
        let purse = spawn_cost_column(commands, fonts, cost, width);
        commands.entity(row).add_child(purse);
    }

    let says = commands
        .spawn((
            Node {
                // `flex_basis: Auto` and not zero, which is what it was. A
                // basis of zero contributes **nothing** to the row's natural
                // width, and a sheet that sizes itself to its content would
                // have measured a row as a keycap plus its pips and come out
                // at [`SHEET_MIN`] however long the sentence was. `Auto` asks
                // for the sentence; `min_width: 0` keeps the row able to
                // shrink back to the cap when the sheet is at [`SHEET_MAX`],
                // which is where the text wraps.
                flex_grow: 1.0,
                flex_basis: Val::Auto,
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if costs.is_none()
        && let Some(cost) = cost
    {
        let title = spawn_cost_title(commands, fonts, cost);
        commands.entity(says).add_child(title);
    }
    let blocks = printed.unwrap_or_else(|| vec![TextBlock::Rules(option.label.clone())]);
    for block in blocks {
        let (words, colour) = match &block {
            TextBlock::Rules(t) => (t.clone(), palette::SLIP_INK),
            TextBlock::Reminder(t) => (t.clone(), palette::SLIP_ASIDE),
        };
        let line = crate::manaui::spawn_rich(commands, fonts, &words, ROW_PT, colour);
        commands.entity(says).add_child(line);
    }
    commands.entity(row).add_child(says);
    commands.entity(row).add_child(keycap);
    row
}

/// The tenth row.
fn spawn_pager(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    page: usize,
    pages: usize,
) -> Entity {
    let row = commands
        .spawn((
            SheetPager,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                padding: UiRect::axes(px(SHEET_PAD_X), px(6)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::tinting_to(Color::NONE, pressed(Color::NONE)),
        ))
        .id();
    let keycap = cap(
        commands,
        fonts,
        &abilitysheet::PAGER.to_string(),
        Color::NONE,
        palette::PARCHMENT_INK,
        ROW_CAP_PT,
    );
    commands.entity(row).add_child(keycap);
    let says = commands
        .spawn((
            Text::new(
                Phrase::SheetMorePage.fill(lang, &[&(page + 1).to_string(), &pages.to_string()]),
            ),
            tf_bold(fonts, 12.0),
            TextColor(palette::SLIP_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(says);
    row
}

/// What a row says: the ability's own printed sentence, or nothing.
///
/// Three things have to line up before the sentence can be drawn and any of
/// them may be missing — the generated table has to know which sentence it is,
/// the permanent has to still carry a printing, and that printing's text has
/// to have arrived, which it has not offline. That is the same triple
/// `stack_sentence` walks.
///
/// `None` rather than the fallback itself, because the *caller* has to know
/// which of the two it got: the fallback label is what the ability costs, and
/// the row draws that again on the right as pips.
///
/// The cost is cut off the front of a real sentence for the same reason — see
/// [`abilitysheet::effect`](baylee_client_core::abilitysheet::effect).
fn row_text(
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    object: ObjectId,
    option: &crate::abilities::AbilityOption,
) -> Option<Vec<TextBlock>> {
    let text = option.printed?;
    let print = duel.view.as_ref()?.object(object)?.card?.print;
    let card = faces.get(print, text.face)?;
    let blocks =
        baylee_client_core::card_face::sentence_blocks(&card.oracle_text, text.line, text.of)?;
    let blocks = abilitysheet::effect(blocks);
    (!blocks.is_empty()).then_some(blocks)
}

/// Follows the card with the sheet that is already built, and puts the nub
/// and the halo where they point at it.
///
/// Every frame, because the card is gliding — see the module header. The
/// write is guarded on where the three pieces already are, so a card standing
/// still costs one comparison and no relayout.
///
/// The anchor is the card's **resting** pose ([`crate::table::CardRest`]) and
/// not the pose it is drawn in. A card under the pointer rises 0.06 units and
/// grows 6%, and an armed one rises further still — so a sheet anchored to
/// what is drawn slid up the screen whenever the hand crossed the permanent
/// it was describing, and again the moment a row was armed. Neither is a
/// *move*: the card is exactly where it was.
pub fn place_ability_sheet(
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    cards: Query<(&crate::table::CardVisual, &crate::table::CardRest)>,
    mut sheet: Query<(
        &mut AbilitySheet,
        &mut Node,
        &mut Visibility,
        &bevy::ui::ComputedNode,
    )>,
    mut nub: Query<(&mut Node, &mut BorderColor), Nub>,
    mut halo: Query<&mut Node, Halo>,
) {
    let Ok((mut sheet, mut node, mut seen, computed)) = sheet.single_mut() else {
        return;
    };
    let (Some(rig), Ok(window)) = (shown.rig(), windows.single()) else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let lens = crate::table::Lens::new(rig, size);
    let Some((mid, card)) = cards
        .iter()
        .find(|(visual, _)| visual.object == sheet.object)
        .and_then(|(_, at)| crate::table::card_box(&lens, &at.0))
    else {
        // The card is not on the table — it left, or the camera cannot see
        // it. Hidden rather than despawned, the way a seat bar is: the sheet
        // is closed by the input path and not by the camera.
        if node.display != Display::None {
            sheet.placed = None;
            node.display = Display::None;
            for (mut piece, _) in &mut nub {
                piece.display = Display::None;
            }
            for mut piece in &mut halo {
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
    if let Ok(mut piece) = halo.single_mut() {
        put_halo(&now, &mut piece);
    }
}

/// Opens and closes the sheet, and despawns it at the end of a close.
///
/// The sheet grows, the nub and the halo follow it once it is nearly there
/// ([`ZOOM_TAIL`]), and the same movement run backwards is what takes them
/// all away. A player who has turned motion off gets the end of it on the
/// first frame, which is the answer `table::glide` and `ShownRig` both give.
///
/// The despawn is here and not in [`sync_ability_sheet`] because a closing
/// sheet has outlived the thing it was about: `Duel::ability_menu` is already
/// `None` and the paper is still on screen.
pub fn zoom_the_sheet(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut sheets: Query<(&mut SheetZoom, &mut UiTransform, &ChildOf), With<AbilitySheet>>,
    mut trim: Query<&mut UiTransform, Trim>,
) {
    let still = prefs.all().reduce_motion;
    for (mut zoom, mut transform, parent) in &mut sheets {
        let span = if zoom.closing { ZOOM_OUT } else { ZOOM_IN };
        zoom.t = if still {
            1.0
        } else {
            (zoom.t + time.delta_secs() / span).min(1.0)
        };
        let done = zoom.t >= 1.0;
        if zoom.closing && done {
            // The whole tree, not the sheet: the nub and the halo are its
            // siblings and the root is what one despawn clears.
            commands.entity(parent.parent()).despawn();
            continue;
        }
        let scale = if zoom.closing {
            // Accelerating away. The opening's overshoot would read as a
            // bounce on the way out, which is a movement asking to be watched
            // by something that is leaving.
            1.0 - (1.0 - ZOOM_FROM) * zoom.t * zoom.t
        } else {
            ZOOM_FROM + (1.0 - ZOOM_FROM) * pop(zoom.t)
        };
        transform.scale = Vec2::splat(scale);

        // The two attached pieces, on their own ramp — written every frame
        // and never guarded on `display`, because the placer is what reveals
        // them and a piece that was skipped while hidden would be shown at
        // whatever scale it was left at for the frame in between.
        let tail = ((zoom.t - ZOOM_TAIL) / (1.0 - ZOOM_TAIL)).clamp(0.0, 1.0);
        for mut piece in &mut trim {
            piece.scale = Vec2::splat(if zoom.closing {
                1.0 - (1.0 - ZOOM_FROM) * zoom.t
            } else {
                tail
            });
        }
    }
}

/// The opening's curve: ease-out-back, running 0 → 1 with an overshoot.
///
/// `1 + (c+1)u³ + cu²` with `u = t − 1`, whose peak is `4c³/27(c+1)²` past
/// the end — see [`ZOOM_BACK`] for why that number and not the usual one.
/// The caller maps the whole curve onto [`ZOOM_FROM`]..1, so the overshoot is
/// that share of the *range* and not of the size.
fn pop(t: f32) -> f32 {
    let u = t - 1.0;
    1.0 + (ZOOM_BACK + 1.0) * u * u * u + ZOOM_BACK * u * u
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
/// The width is measured rather than assumed, the way the height already was:
/// the sheet sizes itself to its own text between [`SHEET_MIN`] and
/// [`SHEET_MAX`], so a constant here would centre a narrow sheet as if it were
/// a wide one and put it off-centre by half the difference.
fn corner_for(mid: Vec2, card: Vec2, sheet: Vec2, window: Vec2) -> Vec2 {
    let above = mid.y - card.y / 2.0 - SHEET_GAP - sheet.y;
    let top = if above >= SHEET_MARGIN {
        above
    } else {
        (mid.y + card.y / 2.0 + SHEET_GAP).min(window.y - SHEET_MARGIN - sheet.y)
    };
    Vec2::new(
        (mid.x - sheet.x / 2.0).clamp(
            SHEET_MARGIN,
            (window.x - SHEET_MARGIN - sheet.x).max(SHEET_MARGIN),
        ),
        top.max(SHEET_MARGIN),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duel's window, measured off the running client.
    const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);
    /// A permanent on a duel's battlefield is about this many logical pixels
    /// wide, and the box keeps the 63:88 card aspect.
    const CARD: Vec2 = Vec2::new(47.0, 65.6);
    /// A sheet of about five rows, at the width a middling sentence asks for.
    const SHEET: Vec2 = Vec2::new(300.0, 210.0);

    #[test]
    fn the_sheet_sits_over_the_card_it_belongs_to() {
        let mid = Vec2::new(864.0, 600.0);
        let at = corner_for(mid, CARD, SHEET, WINDOW);
        assert!(
            (at.x + SHEET.x / 2.0 - mid.x).abs() < 0.5,
            "centred on the card: {at} against {mid}"
        );
        assert!(
            at.y + SHEET.y <= mid.y - CARD.y / 2.0,
            "and clear of its top edge: the sheet ends at {} and the card \
             starts at {}",
            at.y + SHEET.y,
            mid.y - CARD.y / 2.0
        );
    }

    /// The sheet is as wide as its own text now, so every width has to land
    /// on the same card. A constant here would put a narrow sheet off-centre
    /// by half the difference and nothing but the eye would catch it.
    #[test]
    fn a_sheet_of_any_width_is_centred_on_its_card() {
        let mid = Vec2::new(864.0, 600.0);
        for w in [SHEET_MIN, 260.0, 300.0, SHEET_MAX] {
            let at = corner_for(mid, CARD, Vec2::new(w, 210.0), WINDOW);
            assert!(
                (at.x + w / 2.0 - mid.x).abs() < 0.5,
                "a {w}-wide sheet sits at {at}, off the card's {}",
                mid.x
            );
        }
    }

    /// A permanent near the top of the window — an opponent's board — has no
    /// room above it, and the sheet goes under the card rather than off the
    /// screen.
    #[test]
    fn a_sheet_with_no_room_above_the_card_goes_below_it() {
        let mid = Vec2::new(864.0, 90.0);
        let at = corner_for(mid, CARD, SHEET, WINDOW);
        assert!(
            at.y >= mid.y + CARD.y / 2.0,
            "below the card: {} against {}",
            at.y,
            mid.y + CARD.y / 2.0
        );
        assert!(at.y + SHEET.y <= WINDOW.y, "and still inside the window");
    }

    /// Neither edge of the window may cut a row off. A card in the corner is
    /// the ordinary case, not a contrived one: a seat's lands sit at the end
    /// of a lane.
    #[test]
    fn a_sheet_beside_a_card_at_the_edge_stays_inside_the_window() {
        for x in [0.0, 20.0, 1700.0, WINDOW.x] {
            for w in [SHEET_MIN, SHEET_MAX] {
                let sheet = Vec2::new(w, 210.0);
                let at = corner_for(Vec2::new(x, 600.0), CARD, sheet, WINDOW);
                assert!(
                    at.x >= SHEET_MARGIN && at.x + w <= WINDOW.x - SHEET_MARGIN,
                    "at x={x} a {w}-wide sheet's corner is {at}"
                );
            }
        }
    }

    /// A sheet taller than the window it is drawn in still starts inside it.
    /// Nine rows of wrapped rules text on a short window is the shape, and
    /// the alternative is a header scrolled off the top edge.
    #[test]
    fn a_sheet_taller_than_the_window_still_starts_inside_it() {
        let at = corner_for(
            Vec2::new(400.0, 300.0),
            CARD,
            Vec2::new(300.0, 900.0),
            Vec2::new(900.0, 500.0),
        );
        assert!(at.y >= SHEET_MARGIN, "the top is on screen: {at}");
        assert!(at.x >= SHEET_MARGIN, "and so is the left edge");
    }
}

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
mod running {
    use super::*;
    use crate::table::{CameraRig, Canvas, CardVisual, ShownRig, TableCamera, apply_camera_rig};
    use baylee_client_core::layout::TableLayout;
    use baylee_core::ids::PlayerId;

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// An app with the placer, a camera aimed at a duel, a window, and one
    /// card standing in the local seat's creature lane.
    ///
    /// [`apply_camera_rig`] is in the schedule rather than a [`ShownRig`]
    /// being written by hand, because the eased rig is the thing the placer
    /// has to read: a sheet projected from the *target* rig would lead the
    /// camera through every pan.
    fn harness(anchor: ObjectId) -> (App, Entity, Vec2, Vec2) {
        let mut app = App::new();
        let window = app.world_mut().spawn(Window::default()).id();
        let size = {
            let w = app
                .world()
                .entity(window)
                .get::<Window>()
                .expect("a window");
            Vec2::new(w.width(), w.height())
        };
        let canvas = Canvas::hud(size);
        let seats: Vec<_> = (0..2).map(PlayerId::new).collect();
        let table = TableLayout::new(&seats, canvas.aspect(), None);
        let slot = *table.local().expect("a local seat");
        let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
        app.insert_resource(CameraRig::home(&table, canvas))
            .init_resource::<ShownRig>()
            .init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .add_systems(Update, (apply_camera_rig, place_ability_sheet).chain());
        app.world_mut().spawn((TableCamera, Transform::default()));
        let stands =
            Transform::from_translation(crate::table::to_world(at, crate::table::CARD_LIFT))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        app.world_mut().spawn((
            CardVisual {
                object: obj(1),
                count: 1,
            },
            stands,
            // The placer reads this and not the `Transform`, so a card the
            // pointer is on does not drag the paper about. Both are here
            // because a card on the table carries both.
            crate::table::CardRest(stands),
        ));
        let sheet = app
            .world_mut()
            .spawn((
                AbilitySheet {
                    object: anchor,
                    placed: None,
                },
                Node::default(),
                bevy::ui::ComputedNode {
                    size: Vec2::new(300.0, 200.0),
                    ..default()
                },
            ))
            .id();
        (app, sheet, at, size)
    }

    #[test]
    fn the_sheet_is_put_over_the_card_it_is_anchored_to() {
        let (mut app, sheet, at, size) = harness(obj(1));
        app.update();

        let rig = app
            .world()
            .resource::<ShownRig>()
            .rig()
            .expect("the camera was applied");
        // Projected a second time here, from the table point rather than
        // from the card's corners, so this is not the placer's own
        // arithmetic read back to itself.
        let drawn = crate::table::Lens::new(rig, size)
            .project(at)
            .expect("the lane is in front of the eye");

        let node = app.world().entity(sheet).get::<Node>().expect("a node");
        assert_eq!(node.display, Display::Flex, "the sheet is shown");
        let Val::Px(left) = node.left else {
            panic!("the sheet was never placed: {:?}", node.left)
        };
        let Val::Px(top) = node.top else {
            panic!("no top: {:?}", node.top)
        };
        assert!(
            (left + 300.0 / 2.0 - drawn.x).abs() < 1.0,
            "the sheet is centred on the card: its middle is {} and the card \
             is drawn at {}",
            left + 300.0 / 2.0,
            drawn.x
        );
        assert!(
            top + 200.0 < drawn.y && top > 0.0,
            "and stands above it, inside the window: {top} + 200 against {}",
            drawn.y
        );
    }

    /// A sheet whose permanent is not on the table is hidden rather than left
    /// wherever it was last drawn. That is the same answer a seat bar gives
    /// for a shelf the camera cannot see, and for the same reason: the sheet
    /// is closed by the input path and not by the camera.
    #[test]
    fn a_sheet_whose_card_is_not_drawn_is_put_away() {
        let (mut app, sheet, _, _) = harness(obj(9));
        app.update();
        let node = app.world().entity(sheet).get::<Node>().expect("a node");
        assert_eq!(node.display, Display::None);
    }

    /// Nothing on parchment is written in brass.
    ///
    /// `BRASS` on `PARCHMENT` measures 1.9:1 — below every legibility floor —
    /// which is how the zone browser's current tab came to read *fainter*
    /// than the ones beside it. Brass keeps its job as a light: the keycap
    /// on an armed row, the ordering badge, the card glow, each of which sits
    /// on its own fill. The check is on the source because what is being held
    /// is a rule about a whole surface rather than about one node.
    ///
    /// It used to live in `hud::tray` and scan that one file. The tray is a
    /// dark panel now (`docs/redesign-proposal.md` §1.3: parchment is a sheet
    /// you read from, a panel is a place you work in), so the rule moved to
    /// where the parchment actually is — and it reads both surfaces, which is
    /// strictly more than it ever did.
    #[test]
    fn the_parchment_writes_no_letters_in_brass() {
        // Assembled rather than written out, or the needle is in the
        // haystack and this test fails on its own source line.
        let ink_in = format!("TextColor(palette::{}", "BRASS");
        for (what, source) in [
            ("the ability sheet", include_str!("sheet.rs")),
            ("the prompt slip", include_str!("overlay.rs")),
        ] {
            for line in source.lines() {
                let code = line.split("//").next().unwrap_or(line);
                assert!(
                    !code.contains(&ink_in),
                    "brass is a light on {what}, not a letter: {line}"
                );
            }
        }
    }

    /// …and the ink weight of it is what a heading is written in.
    ///
    /// The rule above only says what brass may not do. This is the other half:
    /// the sheet still wants its own accent — the owner asked for the card's
    /// name in the house colour — and [`palette::INK_BRASS`] is that hue taken
    /// down until it carries. Both sides are measured, because a palette entry
    /// nudged for looks is exactly how the unreadable one got there.
    #[test]
    fn the_accent_a_sheet_writes_with_is_dark_enough_to_read() {
        /// sRGB → linear, the transfer function WCAG's ratio is defined over.
        fn linear(c: f32) -> f32 {
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn contrast(a: Color, b: Color) -> f32 {
            let luma = |c: Color| {
                let s = c.to_srgba();
                0.2126f32.mul_add(
                    linear(s.red),
                    0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
                )
            };
            let (one, two) = (luma(a), luma(b));
            (one.max(two) + 0.05) / (one.min(two) + 0.05)
        }

        let light = contrast(palette::BRASS, palette::PARCHMENT);
        assert!(
            light < 2.0,
            "brass is a light and this test's premise is that it cannot be read: {light:.2}:1"
        );
        let ink = contrast(palette::INK_BRASS, palette::PARCHMENT);
        assert!(
            ink >= 4.5,
            "the sheet's accent has to carry body text: {ink:.2}:1"
        );
        // And it is still brass rather than brown: the mix is the same, only
        // the level is lower, so every channel stands in the same ratio.
        let (was, now) = (palette::BRASS.to_srgba(), palette::INK_BRASS.to_srgba());
        let drift = (was.red / was.green - now.red / now.green)
            .abs()
            .max((was.blue / was.green - now.blue / now.green).abs());
        assert!(
            drift < 0.01,
            "the ink drifted off brass's hue by {drift:.3}"
        );
    }
}
