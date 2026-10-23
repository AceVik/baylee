//! The sheet's tree, built: its head, the rows with their costs and pours,
//! the pager and the foot.

#[allow(clippy::wildcard_imports)] // the sheet's own vocabulary
use super::*;

/// One sheet, built.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_sheet(
    commands: &mut Commands,
    fonts: &UiFonts,
    _sheets: &UiSheets,
    faces: &crate::cardtext::CardTexts,
    lang: Lang,
    duel: &Duel,
    object: ObjectId,
    source: Source,
    // The pip half, and empty on a cast sheet: pips, their prefix and the
    // bubble are all questions about what a *permanent* pours, and a card
    // being cast pours nothing. Indexed the same as `rows`.
    options: &[crate::abilities::AbilityOption],
    rows: &[SheetRow],
    split: crate::abilities::Split,
    picked: usize,
    page: usize,
    armed: Option<usize>,
    dismissible: bool,
    fresh: bool,
    standing: Option<Placement>,
) -> Entity {
    // A bubble is the same piece of paper with something else written on it:
    // the grain, the border, the shadow, the nub and the opening are all the
    // sheet's and none of them is worth a second copy. What it
    // does not take is the sheet's *shape* — no head, no footer, no floor
    // under its width, and a row of pips instead of a column of sentences.
    let bubble = crate::abilities::pouring(options);
    let mut node = Node {
        position_type: PositionType::Absolute,
        // As wide as what is on it. An absolutely-positioned node
        // shrink-wraps its content, so the bound is the pair of limits and
        // not a width — and the rows have to be able to *ask* for their
        // natural width for that to mean anything, which is what
        // `flex_basis: Auto` on a row's prose column is for.
        width: Val::Auto,
        // The floor is the sheet's alone. It is there so a list of one-word
        // rows is still a *sheet*; a bubble at that width would be five pips
        // adrift in the middle of a page.
        min_width: px(if bubble { 0.0 } else { SHEET_MIN }),
        max_width: px(SHEET_MAX),
        flex_direction: if bubble {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        },
        // Only the bubble's. A sheet's rows are stretched to the paper's
        // width, which is what lets a row's wash reach both edges.
        align_items: if bubble {
            AlignItems::Center
        } else {
            AlignItems::Stretch
        },
        column_gap: px(POUR_AIR),
        padding: if bubble {
            UiRect::all(px(SHEET_PAD_Y - POUR_RING))
        } else {
            UiRect::vertical(px(SHEET_PAD_Y))
        },
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
            UiTransform::from_scale(Vec2::splat(if fresh { motion::ZOOM_FROM } else { 1.0 })),
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
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
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
    // The glass panel owns its surface; no opaque parchment overlay.

    if bubble {
        // No head and no footer. The head names the permanent, which is
        // directly under the bubble and has not moved; the footer says which
        // key arms and which sends, and nothing here arms — a pip is one
        // press. What is left is the question itself.
        if let Some(prefix) = prefix_of(duel, object, options) {
            let mark = spawn_prefix(commands, fonts, &prefix);
            commands.entity(sheet).add_child(mark);
        }
        for (at, option) in options.iter().enumerate() {
            let Some(pour) = option.pour else { continue };
            let pip = spawn_pour(commands, fonts, at, pour.color, picked == at);
            commands.entity(sheet).add_child(pip);
        }
        return sheet;
    }

    spawn_head(commands, fonts, faces, duel, object, dismissible, sheet);

    // ---- the header of pips ----------------------------------------------
    //
    // A permanent that makes mana *and* does something else puts the colours
    // it can make on one centred row above the list, and the rest of the list
    // is untouched. It stands on every page, because a colour is not an item
    // in the list of things this permanent does — it is the one thing the
    // permanent does that a symbol can say outright.
    if split.pips > 0 {
        let strip = spawn_pour_strip(commands, fonts, duel, object, options, split);
        commands.entity(sheet).add_child(strip);
    }

    // ---- the rows --------------------------------------------------------
    //
    // Whether *this sheet* has a cost column, and how wide, is one answer for
    // all of it and not one per row: the sentences share a left edge, and a
    // row with a column of its own width would put its prose where its
    // neighbours' costs are. Decided over the rows on this page, because a
    // page is what is seen.
    let costs = cost_column(rows, split, page);
    for at in abilitysheet::rows(split.rows, page) {
        let digit = digit_of(at).unwrap_or('?');
        let index = split.option(at);
        let row = spawn_row(
            commands,
            fonts,
            source,
            &rows[index],
            digit,
            armed == Some(index),
            picked == index,
            costs,
        );
        commands.entity(sheet).add_child(row);
    }
    // The pager belongs to the model that has a page counter. `turn_the_page`
    // writes `Duel::ability_page` and `ability_menu_keys` turns it with `0`;
    // a cast list has neither, so a pager here would be a control that does
    // nothing — worse than no pager, because it claims there is more to see
    // and then refuses to show it. See where `page` is decided.
    if source == Source::Ability && abilitysheet::paged(split.rows) {
        let row = spawn_pager(commands, fonts, lang, page, abilitysheet::pages(split.rows));
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
    dismissible: bool,
    sheet: Entity,
) {
    // Two lookups, because the sheet stands beside two kinds of card now and
    // `PlayerView::object` answers for only one of them. A permanent goes
    // through [`crate::face::name_of`], which knows that an ability on the
    // stack borrows its source's name (CR 113.7a); a card in **hand** — which
    // is what a cast chooser is about, and the one zone `object` does not look
    // in — goes through [`crate::face::face_name`], which searches the hand
    // first. Without the second the sheet drew its rows under a blank line.
    let name = duel.view.as_ref().map_or_else(String::new, |view| {
        view.object(object)
            .map(|o| crate::face::name_of(o, view, faces))
            .or_else(|| crate::face::face_name(object, 0, view, Some(faces)))
            .unwrap_or_default()
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
            // register, and `docs/proposals/redesign-proposal.md` §1.3 is what splits
            // the two. On parchment the accent *is* brass — it is what the
            // armed keycap and the armed wash are already made of. At ink
            // weight, though: `BRASS` itself is a light and measures 1.6:1 as
            // letters here, which `the_parchment_writes_no_letters_in_brass`
            // holds.
            tf_bold(fonts, 16.0),
            TextColor(palette::CANDLE),
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

    // A door only where there is somewhere to go. The cross takes the sheet
    // back by clearing the menu that opened it, so on a sheet drawn for a
    // question the **engine** asked there is nothing for it to clear and it
    // would be a control that visibly does nothing — the same objection the
    // pager answers where `Duel::ability_page` is not the counter in play.
    // The name and the hairline stay: those say whose list this is, which is
    // as true of a question that has to be answered as of one that can be put
    // down.
    if dismissible {
        let shut = shut_button(commands, fonts);
        commands.entity(row).add_child(shut);
    }

    commands.entity(sheet).add_child(row);
    let hair = rule(commands, 5.0);
    commands.entity(sheet).add_child(hair);
}

/// The cross in the head, which is one of the four doors out of the sheet.
fn shut_button(commands: &mut Commands, fonts: &UiFonts) -> Entity {
    /// The close button's side.
    ///
    /// Half again the widest keycap and close to the 44 logical pixels the
    /// lobby gives a phone — the sheet is not a responsive screen, it is
    /// pinned to a card 47 px wide, so a target sized for a thumb would be a
    /// sixth of the paper. This is sized for a finger on a tablet, which is
    /// what a card on a table is played with, and it is the only thing on
    /// the sheet a pointer has to *find* rather than being handed by a row.
    const CLOSE: f32 = 30.0;

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
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shut).add_child(cross);
    shut
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
                TextColor(palette::MUTED),
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
            palette::INK,
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
        palette::MUTED,
        // The same grey as the words beside it, so the cap and its sentence
        // are one aside instead of a black key with a quiet label.
        palette::MUTED,
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
pub(super) fn digit_of(at: usize) -> Option<char> {
    u32::try_from(at % abilitysheet::PAGE + 1)
        .ok()
        .and_then(|place| char::from_digit(place, 10))
}

/// [`keycap`] with the sheet's own edge on it.
///
/// The shape is everyone's — see [`keycap`] for why the box is a square and
/// why it grows with its legend. What is the sheet's is the **border**: on
/// parchment a cap filled with brass is ringed in ink and everything else in
/// the soft grey, which is a rule about this paper and about no other
/// surface. It lives here so the caller does not have to know it, and so that
/// the ledge, which draws caps on a dialog, is not silently given it.
fn cap(
    commands: &mut Commands,
    fonts: &UiFonts,
    legend: &str,
    fill: Color,
    ink: Color,
    size: f32,
) -> Entity {
    keycap(
        commands,
        fonts,
        legend,
        fill,
        if fill.alpha() > 0.4 {
            palette::DIALOG
        } else {
            ink
        },
        if fill == palette::BRASS {
            palette::DIALOG
        } else {
            palette::MUTED
        },
        size,
    )
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
            BackgroundColor(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id()
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
fn cost_column(rows: &[SheetRow], split: crate::abilities::Split, page: usize) -> Option<f32> {
    let mut widest: Option<f32> = None;
    for row in abilitysheet::rows(split.rows, page) {
        let at = split.option(row);
        let Some(cost) = rows[at].cost.as_deref() else {
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
            palette::MUTED,
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
    let line =
        crate::manaui::spawn_rich_marks(commands, fonts, cost, ROW_PT, COST_MARK, palette::MUTED);
    commands.entity(holder).add_child(line);
    let hair = commands
        .spawn((
            Node {
                height: px(1),
                margin: UiRect::top(px(COST_TITLE_AIR)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LINE.with_alpha(COST_RULE_WASH)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(holder).add_child(hair);
    holder
}

/// The colours a permanent can make, centred above the list of what else it
/// does.
///
/// The same pips a bubble is made of, on a sheet that also has sentences on
/// it. It is a **header** and not a row: it carries no digit and no wash that
/// says a state, it is drawn on every page, and it is centred where a row's
/// ink starts at the paper's left edge — all four say the same thing, which is
/// that the list below it is a list and this is not an item in it.
///
/// The band it sits on is the fourth, and the one the owner asked for: see
/// [`POUR_BAND_WASH`] for why a *structural* wash is the only register that
/// could carry it.
fn spawn_pour_strip(
    commands: &mut Commands,
    fonts: &UiFonts,
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
    split: crate::abilities::Split,
) -> Entity {
    let strip = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: px(POUR_AIR),
                padding: UiRect::vertical(px(POUR_BAND_AIR)),
                // Full bleed, which is by construction rather than by a
                // negative margin: the sheet has no horizontal padding — its
                // rows carry their own — and stretches its children, so a
                // background here reaches both edges of the paper.
                //
                // Above it, the paper between the head's hairline and the
                // band; below it, the air against the first sentence, which
                // is what it always was.
                margin: UiRect::new(px(0), px(0), px(POUR_BAND_GAP), px(SHEET_PAD_Y - POUR_RING)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LINE.with_alpha(POUR_BAND_WASH)),
            Pickable::IGNORE,
        ))
        .id();
    // The same rule the bubble asks: a header of pips that all stand for one
    // tap whose pour is not a number says how many as well as which colour.
    // On a sheet it is ordinarily `None` — the tap the pips won is the one a
    // player can predict, which is what [`crate::abilities::pour_out`] gives
    // them to — and it is asked anyway, because "which pips" is the same
    // question in both places and two answers to it would drift.
    if let Some(prefix) = prefix_of(duel, object, &options[..split.pips]) {
        let mark = spawn_prefix(commands, fonts, &prefix);
        commands.entity(strip).add_child(mark);
    }
    for (at, option) in options.iter().enumerate().take(split.pips) {
        let Some(pour) = option.pour else { continue };
        let pip = spawn_pour(commands, fonts, at, pour.color, duel.ability_pick == at);
        commands.entity(strip).add_child(pip);
    }
    strip
}

/// What stands before a row of pips, if anything does.
///
/// [`crate::abilities::bubble_prefix`] is the rule; this is only the lookup
/// that gets a `PlayerView` to it, so that both places that draw pips — the
/// bubble and the strip on a sheet — ask the one question rather than each
/// deciding for themselves.
fn prefix_of(
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
) -> Option<String> {
    crate::abilities::bubble_prefix(duel.view.as_ref()?, object, options)
}

/// `X×`, set before the pips it multiplies.
///
/// Ink and not brass: it is part of the question, at the weight the row's own
/// words are set in, and the two registers that are not ink on this sheet are
/// both *states* — see [`PICK_MARK`]. Set at the pip's own size because that
/// is what it is a multiplier on, and `Pickable::IGNORE` because it is the
/// one thing on a bubble that is not a control.
fn spawn_prefix(commands: &mut Commands, fonts: &UiFonts, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text.to_string()),
            crate::hud::tf(fonts, POUR_MARK * 0.8),
            TextColor(palette::INK),
            Node {
                // The bubble's own `column_gap` is the air between two pips,
                // which is too little between a word and a symbol.
                margin: UiRect::right(px(POUR_AIR)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// One pip of a mana bubble: the colour, and the whole of the control.
///
/// There is no keycap and no sentence beside it, which is the point — a mana
/// symbol is the one label in this interface that needs no translating and no
/// reading. On a bubble the digit that sends it is the pip's own position and
/// is simply not drawn; on a sheet it has no digit at all
/// ([`crate::abilities::Split`]). Either way a number printed beside a `{W}`
/// would be a second thing to look at on a control whose entire content is one
/// mark.
///
/// The button is the disc plus [`POUR_RING`] of paper on every side. The disc
/// itself is `Pickable::IGNORE` (see [`crate::manaui::spawn_pip`]), so the
/// press lands here however precisely the pointer hit the mark.
fn spawn_pour(
    commands: &mut Commands,
    fonts: &UiFonts,
    index: usize,
    color: baylee_core::mana::ManaColor,
    picked: bool,
) -> Entity {
    let button = commands
        .spawn((
            crate::hud::AbilityButton { index },
            Node {
                // The ring is cut *out of* the paper round the disc, not added
                // to it, so the target stays exactly [`POUR_RING`] deep
                // whether the cursor is here or not.
                border: UiRect::all(px(PICK_RING)),
                padding: UiRect::all(px(POUR_RING - PICK_RING)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(POUR_MARK / 2.0 + POUR_RING)),
                ..default()
            },
            // No wash under a pip. The ring is the control's extent, and brass
            // at [`PICKED_WASH`]'s weight over [`POUR_BAND_WASH`] composites
            // to 191,172,125 against the band's own 192,175,135 — which is
            // brass disappearing into brass, and the whole of what the owner
            // could not see.
            BackgroundColor(Color::NONE),
            BorderColor::all(if picked { PICK_MARK } else { Color::NONE }),
            // **A lift, where a row is a tint.** This said the opposite for
            // one commit — "a button that grew under the pointer would re-lay
            // that glyph out" — and that is not what a lift does: `Feel`
            // writes `UiTransform::scale`, which is applied after layout, so
            // the disc, the clip and the glyph inside it are the same laid-out
            // node drawn larger. What is true of a *row* is not true here for
            // the reason [`POUR_LIFT`] gives: a row is a line of writing, and
            // a pip is one mark with air around it.
            //
            // The ink under it stays and is what the movement is read
            // against: [`pressed`] on a resting pip is plain ink at
            // [`ROW_PRESS`], the register the whole sheet already answers a
            // pointer in — and it is the same ink whether the cursor is on
            // this pip or not, because the ring is not `Feel`'s to write.
            Feel::lifting(Color::NONE, pressed(Color::NONE), POUR_LIFT),
        ))
        .id();
    let pip = crate::manaui::spawn_pip(
        commands,
        fonts,
        baylee_client_core::manapip::of_mana(color),
        POUR_MARK,
    );
    commands.entity(button).add_child(pip);
    button
}

/// One row: what it costs, what the ability does, the key that arms it.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub(super) fn spawn_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    source: Source,
    option: &SheetRow,
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
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                // The bar is taken out of the row's own left padding, so the
                // sentence starts where it always did — see [`PICK_BAR`] for
                // why every row carries the border and not only this one.
                border: UiRect::left(px(PICK_BAR)),
                padding: UiRect::new(px(SHEET_PAD_X - PICK_BAR), px(SHEET_PAD_X), px(6), px(6)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(wash),
            // The cursor, in the one register `Feel` does not write — so an
            // armed row that the keyboard is also on says both at once, which
            // is the common case (press `2`, and the cursor is on 2).
            BorderColor::all(if picked { PICK_MARK } else { Color::NONE }),
            // Lit but not lifted. A row is a *sentence* with a wash behind it,
            // and a button's 2.5% grow-on-hover reflows that sentence every
            // time the pointer crosses it — which the owner read as the text
            // changing size, because that is exactly what it is.
            Feel::tinting_to(wash, pressed(wash)),
        ))
        .id();
    // The *whole row* is the button, not the keycap on it: a target 21 pixels
    // across beside a sentence that is not clickable is a row a player aims at
    // and misses.
    //
    // Which button it is, is the whole of what [`Source`] decides. An ability
    // row is armed and then sent and is an [`crate::hud::AbilityButton`]; a
    // cast row answers by index and is one row of an indexed choice, which
    // [`crate::hud::ChoiceButton`] already is everywhere else in this client.
    // Inserted after the spawn rather than chosen inside the tuple, because a
    // tuple of components cannot hold an either-or.
    match source {
        Source::Granted(id) => {
            commands.entity(row).insert(crate::hud::MenuButton {
                action: crate::hud::MenuAction::PickGranted(id),
            });
        }
        Source::Ability => {
            commands.entity(row).insert(crate::hud::AbilityButton {
                index: option.answer,
            });
        }
        Source::Cast => {
            commands.entity(row).insert(crate::hud::ChoiceButton {
                decision_id: None,
                index: option.answer,
            });
        }
    }

    let keycap = cap(
        commands,
        fonts,
        &digit.to_string(),
        if armed {
            palette::BRASS
        } else {
            palette::MUTED
        },
        // Dark on gold in both states. White on brass fails contrast, and an
        // armed row is the one a player is about to commit to.
        palette::INK,
        ROW_CAP_PT,
    );
    // What the ability *does*, in the card's own words or not at all.
    //
    // There used to be a second source here — `abilities::effect_label`, a
    // table of short categories composed in this client ("Scry", "Draw
    // cards") for the rows whose printed text had not arrived. It is gone,
    // and the owner's instruction is the whole reason: *„Bitte nicht custom
    // texte für abilities verwenden, sondern die echten texte von
    // skryfall."* A composed category is a second wording of a card that
    // already has one, it is never the one the player reads on the physical
    // card, and it exists only where the real text is missing — which is a
    // fetch to repair, not a sentence to invent. `cardtext` now falls back
    // to Scryfall itself when the gateway has nothing, so the missing case
    // is missing for a moment rather than for a game.
    //
    // The cost is the same sentence's own head, cut off it by
    // `abilitysheet::cut` and in the same words: the card's, in the player's
    // language when their text pairs. A sentence drawn whole says its cost
    // itself and has an empty column, and only a row the card prints no
    // sentence for draws the ability's symbols there instead.
    let printed = option.blocks.as_deref();
    let cost = option.cost.as_deref();
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
    // No printed text and nothing invented to stand in for it: the row is
    // its cost and its key. `option.label` is not a third source — for an
    // activated ability it is the cost's symbols, already drawn a few pixels
    // to the left, and drawing it again was a row saying one badge twice and
    // nothing else, 126 times over the pool. Which
    // sentence a row *is* is swept over the whole pool by
    // `every_written_row_knows_which_printed_sentence_it_is`.
    match printed {
        Some(blocks) => {
            for block in blocks {
                let (words, colour) = match block {
                    TextBlock::Rules(t) => (t.clone(), palette::INK),
                    TextBlock::Reminder(t) => (t.clone(), palette::MUTED),
                };
                let line = crate::manaui::spawn_rich(commands, fonts, &words, ROW_PT, colour);
                commands.entity(says).add_child(line);
            }
        }
        // Nothing printed. Whether there is a line to write instead was
        // decided where the row was built and is [`SheetRow::line`]: for an
        // ability it is set only for the three things a permanent can offer
        // that are on no card at all, and for a cast row it is the card's own
        // sentence or the face's own name. A row with neither draws its cost
        // and its key and waits — the text is a fetch away, and inventing a
        // stand-in for the second it is missing is what put three different
        // wordings of one card in front of the owner.
        None => {
            if let Some(text) = option.line.as_deref() {
                let line = crate::manaui::spawn_rich(commands, fonts, text, ROW_PT, palette::INK);
                commands.entity(says).add_child(line);
            }
        }
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
                // The cursor never lands here, and the border is carried all
                // the same: see [`PICK_BAR`]. A pager three pixels narrower
                // than the rows above it is a bent left edge.
                border: UiRect::left(px(PICK_BAR)),
                padding: UiRect::new(px(SHEET_PAD_X - PICK_BAR), px(SHEET_PAD_X), px(6), px(6)),
                border_radius: BorderRadius::all(px(KEYCAP_R)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Feel::tinting_to(Color::NONE, pressed(Color::NONE)),
        ))
        .id();
    let keycap = cap(
        commands,
        fonts,
        &abilitysheet::PAGER.to_string(),
        Color::NONE,
        palette::INK,
        ROW_CAP_PT,
    );
    commands.entity(row).add_child(keycap);
    let says = commands
        .spawn((
            Text::new(
                Phrase::SheetMorePage.fill(lang, &[&(page + 1).to_string(), &pages.to_string()]),
            ),
            tf_bold(fonts, 12.0),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(says);
    row
}
