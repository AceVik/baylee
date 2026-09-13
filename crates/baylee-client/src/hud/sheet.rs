//! The ability sheet: what a permanent can do, on parchment, beside the card.
//!
//! It replaces a row of buttons in the prompt bar. Those buttons carried the
//! ability's *cost* as their whole label — `{2}, {T}` — because a button on a
//! bar has room for four words, so a player read what an ability charged and
//! never what it did, and the bar was at the bottom of the window while the
//! permanent was on the table.
//!
//! What is here is a sheet of paper laid beside the card it belongs to: the
//! permanent's name, a numbered row per thing it can do with the ability's own
//! printed sentence on it, and the cost as pips on the right where a cost
//! belongs. `docs/redesign-proposal.md` §7 is the design.
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
const SHEET_GAP: f32 = 10.0;

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

/// The wash under the row that is armed — [`palette::BRASS`] at 16%.
///
/// It sits on an opaque sheet and not on felt, which is the whole reason a
/// translucent fill is allowed here at all: parchment over dark cloth goes
/// grey, and parchment over parchment is warmer parchment.
const ARMED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.16);

/// The wash under an armed row with the pointer on it.
///
/// Brighter than [`ARMED_WASH`] and not dimmer: `Feel` crossfades from the
/// resting colour to this one, so a hot colour below the base would make
/// hovering the row a player has already committed to *take light away*.
const ARMED_HOT: Color = Color::srgba(0.788, 0.635, 0.153, 0.24);

/// The wash under the row the keyboard is on.
///
/// The same claim at half the weight, because the two are different claims
/// about the same row: the cursor is *where a key would land* and the arming
/// is *what a key has already done*.
const PICKED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.08);

/// The parent of the sheet, so one despawn clears it.
#[derive(Component)]
pub struct AbilitySheetRoot;

/// The sheet, and where it was last put.
#[derive(Component)]
pub struct AbilitySheet {
    /// The permanent it belongs to.
    pub object: ObjectId,
    /// The corner it is already at, so a camera standing still costs one
    /// comparison instead of a relayout of the whole sheet.
    placed: Option<Vec2>,
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
        if revision.object.is_some() {
            *revision = SheetRevision::default();
            for entity in &existing {
                commands.entity(entity).despawn();
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
    );
    commands.entity(root).add_child(sheet);
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
) -> Entity {
    let sheet = commands
        .spawn((
            AbilitySheet {
                object,
                placed: None,
            },
            Node {
                position_type: PositionType::Absolute,
                // As wide as what is on it. An absolutely-positioned node
                // shrink-wraps its content, so the bound is the pair of
                // limits and not a width — and the rows have to be able to
                // *ask* for their natural width for that to mean anything,
                // which is what `flex_basis: Auto` on a row's prose column is
                // for.
                width: Val::Auto,
                min_width: px(SHEET_MIN),
                max_width: px(SHEET_MAX),
                flex_direction: FlexDirection::Column,
                padding: UiRect::vertical(px(SHEET_PAD_Y)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT),
            BorderColor::all(palette::PARCHMENT_EDGE),
            BoxShadow::new(
                palette::SHEET_SHADOW,
                Val::Px(0.0),
                Val::Px(6.0),
                Val::Px(0.0),
                Val::Px(18.0),
            ),
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
    /// Larger than a keycap and smaller than the 44 logical pixels the lobby
    /// gives a phone. The sheet is not a responsive screen — it is pinned to
    /// a card 47 px wide — so a target sized for a thumb would be a third of
    /// the paper's width; this is sized for a finger on a tablet, which is
    /// what a card on a table is played with.
    const CLOSE: f32 = 24.0;

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
            BackgroundColor(Color::NONE),
            Feel::rising_to(Color::NONE, PICKED_WASH),
        ))
        .id();
    let cross = commands
        .spawn((
            // `×` (U+00D7), which Alegreya Sans' Bold cut carries — the
            // dedicated multiplication and ballot crosses (U+2715, U+2716)
            // are not in the family and would draw as tofu.
            Text::new("\u{d7}"),
            tf_bold(fonts, 15.0),
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

/// One row: the keycap, what the ability does, what it costs.
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
            Feel::rising_to(wash, if armed { ARMED_HOT } else { PICKED_WASH }),
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
        11.0,
    );
    commands.entity(row).add_child(keycap);

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
    // What the ability *does*, if the card's text is here to say it, and what
    // it costs if it is not: the fallback label is `printed_label`'s answer,
    // which is the cost.
    let printed = row_text(faces, duel, object, option);
    let blocks = printed
        .clone()
        .unwrap_or_else(|| vec![TextBlock::Rules(option.label.clone())]);
    for block in blocks {
        let (words, colour) = match &block {
            TextBlock::Rules(t) => (t.clone(), palette::SLIP_INK),
            TextBlock::Reminder(t) => (t.clone(), palette::SLIP_ASIDE),
        };
        let line = crate::manaui::spawn_rich(commands, fonts, &words, 13.0, colour);
        commands.entity(says).add_child(line);
    }
    commands.entity(row).add_child(says);

    // The pip column is skipped where it would print the row's own words a
    // second time. Offline there is no card text at all, so every printed
    // ability falls back to its cost — and a row reading `{2}, {T}` on the
    // left and `{2}, {T}` on the right is the duplication the sentence-first
    // layout exists to remove, not a cost drawn where a cost belongs.
    let repeats = printed.is_none() && option.cost.as_deref() == Some(option.label.as_str());
    if let Some(cost) = option.cost.as_deref().filter(|_| !repeats) {
        let pips = crate::manaui::spawn_rich(commands, fonts, cost, 13.0, palette::SLIP_SOFT);
        commands.entity(row).add_child(pips);
    }
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
            Feel::rising_to(Color::NONE, PICKED_WASH),
        ))
        .id();
    let keycap = cap(
        commands,
        fonts,
        &abilitysheet::PAGER.to_string(),
        Color::NONE,
        palette::PARCHMENT_INK,
        11.0,
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

/// Follows the card with the sheet that is already built.
///
/// Every frame, because the card is gliding — see the module header. The
/// write is guarded on where the sheet already is, so a card standing still
/// costs one comparison and no relayout.
pub fn place_ability_sheet(
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    cards: Query<(&crate::table::CardVisual, &Transform)>,
    mut sheet: Query<(&mut AbilitySheet, &mut Node, &bevy::ui::ComputedNode)>,
) {
    let Ok((mut sheet, mut node, computed)) = sheet.single_mut() else {
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
        .and_then(|(_, at)| crate::table::card_box(&lens, at))
    else {
        // The card is not on the table — it left, or the camera cannot see
        // it. Hidden rather than despawned, the way a seat bar is: the sheet
        // is closed by the input path and not by the camera.
        if node.display != Display::None {
            sheet.placed = None;
            node.display = Display::None;
        }
        return;
    };
    // `ComputedNode` is bevy_ui's own layout, which is a frame old here for
    // the reason the module header gives. A frame is nothing to a sheet that
    // stands for as long as a player is reading it, and it is the only thing
    // that knows how tall — and, since the width became the text's to decide,
    // how wide — a sheet of text came out.
    let sheet_box = computed.size() * computed.inverse_scale_factor;
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
    if sheet.placed == Some(corner) && node.display == Display::Flex {
        return;
    }
    sheet.placed = Some(corner);
    node.display = Display::Flex;
    node.left = px(corner.x);
    node.top = px(corner.y);
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
        app.world_mut().spawn((
            CardVisual {
                object: obj(1),
                count: 1,
            },
            Transform::from_translation(crate::table::to_world(at, crate::table::CARD_LIFT))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
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
