//! The ledge: the top edge of the hand zone, and the place a player answers
//! the game from.
//!
//! It is named for `tabletop::MAT_LEDGE`, the shoulder of a seat's mat that
//! the seat bar is written along. The hand zone gets the same shoulder, at
//! its top — the mana pool at one end, the engine's question and its answers
//! in the middle, the two ways to leave the game at the other. Nothing here
//! floats: the four things that used to hover over the table are one edge.
//!
//! This file holds the shelf and the middle of it: the question, its answers
//! and the armed card that replaces them. The mana pool and the two ways out
//! arrive in their own steps — their columns are already here and already the
//! width they will be, so that nothing the middle does moves when they land.
//!
//! Everything taller than one line belongs in the drawer, which is a step
//! further still; until it exists, the rows that were going there are left on
//! what remains of the prompt slip (`overlay::leftover_slip`) rather than
//! deleted.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The shelf itself: the one node in the overlay's retained tree that
/// **outlives a rebuild**.
///
/// [`super::HudRevision`] counts the hover, so `sync_overlay` tears its tree
/// down and builds it again whenever the pointer crosses a card — hundreds of
/// times a turn. Everything in that tree is content that follows the pointer
/// (the preview *is* the hover) or is cheap enough not to care. The shelf is
/// neither: it is the edge the window ends at, it is there at the first frame
/// and never goes, and the buttons on it carry a [`crate::ambience::Feel`]
/// whose warmth is state on the entity — a shelf rebuilt under the pointer
/// snaps the button the player is reaching for back to rest.
///
/// So `sync_overlay` keeps the root and this node across a rebuild and
/// despawns the rest, and the shelf's own contents answer to their own
/// revision. It is **not** a second root, and that is a measurement rather
/// than a preference: a root sorts wholesale against the others, and the
/// shelf has to stand *over* the table veil (`Z_VEIL`, the whole window) and
/// *under* the hover preview (`Z_PREVIEW`, which `hand::beside` may put over
/// the shelf when it describes a card near the bottom of the screen). Both
/// are children of [`HudRoot`], so the shelf has to be one too.
#[derive(Component)]
pub struct LedgeShelf;

/// Spawns the shelf: opaque, full width, [`hand::LEDGE_H`] tall, standing on
/// the top of the hand zone.
///
/// **A sibling of the zone and not a child of it**, which is the one thing
/// about this node that is not free to change. `ZIndex` counts among
/// siblings, and the zone sits at [`Z_HAND`] — under the table veil the zone
/// dialog paints over the whole window at [`Z_VEIL`], deliberately, because
/// the hand cannot answer what that dialog is asking. The question *can*, and
/// a question drawn dimmed is a question the player is being told not to
/// answer. The slip stood above that veil for exactly this reason and the
/// ledge inherits its place.
///
/// Two more things that look like details and are not. The shelf is
/// **opaque**: it is the edge the table ends at, and a translucent one reads
/// as another veil rather than as a shelf. And its overflow is **visible**:
/// the drawer grows up out of it, so a `clip()` copied from the zone below
/// would leave a drawer nobody can see.
pub(super) fn spawn_ledge(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            LedgeShelf,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H),
                left: px(0),
                right: px(0),
                height: px(hand::LEDGE_H),
                // The lip is paid for out of the **top** padding, which is why
                // this is not `UiRect::axes`. `BoxSizing::DEFAULT` is
                // `BorderBox`, so `LEDGE_H` is the outside of the shelf and
                // the border eats into it: 6 + 28 + 6 is 40 only if the line
                // is not there, and with it a 28-px button overflows by one.
                // Taking the pixel off the padding rather than adding it to
                // the shelf keeps the rule the whole height budget is built on
                // — every pixel of ledge is a pixel of table — and costs
                // nothing to look at, because 1 + 5 above the button reads as
                // the 6 below it.
                padding: UiRect::new(px(EDGE), px(EDGE), px(LEDGE_PAD_Y - LIP), px(LEDGE_PAD_Y)),
                // The lip: one line along the top and nothing down the sides
                // or under it, because the zone runs on to the window's own
                // edges and an edge has no corners.
                border: UiRect::top(px(LIP)),
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            // Downwards, onto the cards. The comment on the zone forbids a
            // shadow there and means it — a shadow is drawn from a node's
            // rectangle, so a transparent node with one lays a hard band
            // across the table. This node is opaque and its rectangle *is*
            // the shelf, so the shadow falls where a shelf's shadow falls and
            // is what turns a card running under it into a card on a shelf
            // rather than a card cut by a line.
            BoxShadow(vec![ShadowStyle {
                color: palette::SHADOW,
                x_offset: px(0.0),
                y_offset: px(4.0),
                spread_radius: px(0.0),
                blur_radius: px(12.0),
            }]),
            ZIndex(Z_LEDGE),
            // The shelf itself answers nothing and must not swallow a click
            // meant for the table — but its children are buttons, and a
            // button in a node the pointer cannot see is a button that cannot
            // be pressed. Hoverable, blocking nothing, exactly as the zone
            // below it is.
            //
            // `should_block_lower: false` on an *opaque* node does mean a
            // click on the bare shelf reaches whatever 3D geometry is behind
            // it, which is not obviously right. It is harmless today because
            // the only thing down there is the slab's margin and nothing on
            // it is pickable; if the layout ever puts a card under the shelf,
            // this is the line that has to change.
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id()
}

/// The air above and below a button on the shelf.
///
/// Twice this plus a button's [`BUTTON_H`] is [`hand::LEDGE_H`], and that
/// equation is the only reason either number is what it is. The [`LIP`] comes
/// out of the top of it rather than out of the shelf; the node says why.
pub(super) const LEDGE_PAD_Y: f32 = 6.0;

/// The line along the top of the shelf, where the table stops.
pub(super) const LIP: f32 = 1.0;

/// How tall anything a player presses on the shelf is.
///
/// A keycap with the button's own air around it: [`KEYCAP_SIDE`] is 1.9 and
/// [`CAP_PT`] is 10.5, so the cap is a 20-pixel square and `20 + 2 · 4` is
/// this. Written out rather than computed all the same, because the equation
/// below is the one that actually holds it — a cap drawn at another size
/// would have to leave the shelf's height alone, not move it.
///
/// Not the 44 a phone would ask for: this is a desktop table, the shelf is
/// 40 px of a window that would rather be table, and a 44-px target would take
/// another 16 px off the hand. A deliberate refusal, written down as one.
pub(super) const BUTTON_H: f32 = 28.0;

/// The shelf's arithmetic, as one statement rather than four comments.
const _: () = assert!(LEDGE_PAD_Y * 2.0 + BUTTON_H == hand::LEDGE_H);

// ------------------------------------------------------- what stands on it
//
// Three absolutely positioned children rather than one flex row with
// `space-between`, because the question belongs on the **window's** centre —
// which is the middle of the player's own mat, where their eyes already are
// — and `space-between` would put it in the middle of whatever room the
// other two columns left over, so it would shuffle sideways every time a
// mana pip arrived. `docs`' AX §2.3 has the argument and the widths.
//
// One thing about absolute children here is a taffy fact rather than a
// choice: `perform_absolute_layout_on_absolute_children` measures an inset
// against `container_size - border`, and **not** minus padding. So the
// shelf's own `padding` does nothing for these three, and each states the
// band it stands in out of the same two constants the shelf is built from.

/// Which snapshot the shelf currently shows.
///
/// Its own counter and not [`super::HudRevision`], which counts the hover: the
/// shelf would otherwise be rebuilt every time the pointer crossed a card,
/// hundreds of times a turn, and a button's [`Feel`] would lose its warmth
/// under the player's own pointer. [`LedgeShelf`] is what makes that possible
/// — it outlives the rebuild the rest of the overlay goes through.
///
/// Compared **whole** rather than field by field, which is the one way this
/// differs from [`super::HudRevision`] and the reason is that struct's own
/// test: a field there can be compared and never assigned (the tree redraws
/// every frame) or assigned and never compared (a control that silently does
/// nothing), and `hud/tests.rs` reads the source to catch both. `PartialEq`
/// and one assignment cannot express either mistake, so there is nothing for
/// such a test to find.
///
/// There is no `hovered` here and there must not be. That is the whole point
/// of the struct, and `the_shelf_does_not_follow_the_pointer` holds it.
#[derive(Resource, Default, Clone, PartialEq)]
pub struct LedgeRevision {
    /// Which snapshot of the game.
    pub(super) seq: Option<u64>,
    /// The question, as the sentence says it.
    pub(super) prompt: Option<String>,
    /// The engine's refusal, which replaces that sentence.
    pub(super) error: Option<String>,
    /// What the connection has to say, which replaces it first.
    pub(super) link_note: Option<baylee_client_core::i18n::Phrase>,
    /// Whether this seat is being asked at all — the sentence's weight, and
    /// whether there are answers under it.
    pub(super) waiting: bool,
    /// Whether the zone browser's dialog is holding the answer, which is what
    /// keeps a second Confirm off the shelf.
    pub(super) elsewhere: bool,
    /// What has been picked so far, because `can_confirm` reads it and the
    /// lone OK answer appears the moment it turns true.
    pub(super) selected: Vec<ObjectId>,
    /// What is armed and waiting for its second press. It never leaves the
    /// client, so nothing else here moves when it changes.
    pub(super) armed: Option<crate::Armed>,
    /// Whether the client's own cast chooser is standing, which takes the
    /// answers away: the engine's window behind it is an ordinary priority,
    /// and "Pass priority" under "Choose how it is cast" is two primary
    /// answers saying opposite things.
    pub(super) cast_menu: bool,
    /// The language the shelf is written in.
    pub(super) lang: Option<Lang>,
    /// Every keycap's legend comes out of the keymap, so a rebinding has to
    /// reach the shelf on the next frame rather than at the next question.
    pub(super) keys: Option<baylee_client_core::prefs::Keymap>,
    /// The window's width, rounded to whole pixels: it is what decides
    /// whether the question keeps its keycaps.
    pub(super) window_w: i32,
}

/// The size the shelf's own prose is set at.
const SENTENCE_PT: f32 = 14.0;

/// The size an answer's label is set at.
const LABEL_PT: f32 = 13.0;

/// The size a keycap's legend is set at, which is what sets the cap's side:
/// `10.5 · KEYCAP_SIDE` is the 20-pixel square §4.2 asks for.
const CAP_PT: f32 = 10.5;

/// Between a keycap and the words it belongs to.
const CAP_GAP: f32 = 6.0;

/// An answer's own air, left and right of its contents.
const BUTTON_PAD_X: f32 = 10.0;

/// And above and below, which the button's [`BUTTON_H`] mostly settles: with
/// `BoxSizing::BorderBox` a 28-pixel outside and a 1-pixel border leave 26,
/// and this is what a 13-point line is given of it.
const BUTTON_PAD_Y: f32 = 4.0;

/// What the left column will take once the mana pool moves onto the shelf.
///
/// **Reserved, not measured**, and deliberately at §2.3's *worst* case: the
/// label, six entries and the gaps between them, plus [`EDGE`]. §2.3 refuses
/// to centre the question between its neighbours precisely so that it does
/// not move when a mana pip arrives — and a reservation that followed the
/// pool would be that refusal undone one level down. Holding the worst case
/// from the start also means the step that draws the pool moves ink into
/// room that is already there and changes no measurement.
const LEFT_RESERVED: f32 = 325.0;

/// The same for the right column: two buttons and the edge, §2.3's figure.
const RIGHT_RESERVED: f32 = 214.0;

/// Builds what stands on the shelf, when what is written on it changes.
///
/// After `sync_overlay`, because the shelf it fills is spawned there — and
/// the emptiness check below is what makes that ordering a preference rather
/// than a requirement: a shelf that has just been built afresh is filled even
/// when nothing in the revision moved.
#[allow(clippy::too_many_lines)] // one retained-UI rebuild, sectioned by comments
#[allow(clippy::too_many_arguments)]
pub fn sync_ledge(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<LedgeRevision>,
    shelf: Query<(Entity, Option<&Children>), With<LedgeShelf>>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    windows: Query<&Window>,
) {
    let Ok((shelf, standing)) = shelf.single() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    // A finished game is the one question this shelf does not answer: who won
    // is the end screen's whole subject, set at four times the size. The
    // column simply empties — see `sync_overlay`'s own note, which this is
    // the other half of.
    let over = duel.ending().is_some();
    let turn = duel
        .view
        .as_ref()
        .map_or(baylee_client_core::Turn::Mine, |v| {
            baylee_client_core::Turn::of(v.active, v.seat)
        });
    let waiting = !duel.is_my_turn_to_act();
    let elsewhere = duel.browser.answers_here(duel.interaction.as_ref());
    let prompt = duel
        .cast_menu
        .as_ref()
        .filter(|_| !over)
        .map(|m| m.prompt().headline(lang, turn, duel.statics.as_ref()))
        .or_else(|| {
            duel.interaction
                .as_ref()
                .filter(|_| !over)
                .map(|i| i.prompt().headline(lang, turn, duel.statics.as_ref()))
        });
    #[allow(clippy::cast_possible_truncation)]
    let window_w = windows.single().map_or(1200, |w| w.width() as i32);
    let next = LedgeRevision {
        seq: duel.board.as_ref().map(|b| b.seq),
        prompt,
        error: duel.last_error.clone().filter(|_| !over),
        link_note: duel.link_note.filter(|_| !over),
        waiting,
        elsewhere,
        selected: duel
            .interaction
            .as_ref()
            .map(|i| i.selected().collect())
            .unwrap_or_default(),
        armed: duel.armed.clone(),
        cast_menu: duel.cast_menu.is_some() && !over,
        lang: Some(lang),
        keys: Some(prefs.keymap().clone()),
        window_w,
    };
    // The second half of the gate is what covers a shelf that was spawned
    // afresh with the revision still describing the tree before it: the
    // columns are always three, so an empty shelf is one nothing has filled.
    if *revision == next && standing.is_some_and(|c| !c.is_empty()) {
        return;
    }
    *revision = next;

    for child in standing.into_iter().flatten() {
        commands.entity(*child).despawn();
    }

    // The middle is built first because it is the only one that knows how
    // wide it is, and how wide it is decides the arrangement of all three.
    let answers = answers_for(&duel, lang, over, waiting, elsewhere);
    let armed = duel
        .armed
        .as_ref()
        .filter(|_| !over)
        .and_then(|a| super::overlay::armed_label(&duel, lang, a));
    let sentence = revision
        .link_note
        .map(|note| (note.text(lang).to_string(), true))
        .or_else(|| revision.error.clone().map(|text| (text, true)))
        // An armed card says what it is about to do on the button itself, so
        // the question above it would be the same sentence a second time —
        // §6: the shelf never shows two sentences, and the armed row is the
        // one state that takes the sentence away rather than replacing it.
        .or_else(|| {
            revision
                .prompt
                .clone()
                .filter(|_| armed.is_none())
                .map(|text| (text, false))
        });

    let caps = keys_for(&prefs, &answers, armed.is_some());
    let caps_w: f32 = caps.iter().flatten().map(|c| cap_width(c) + CAP_GAP).sum();
    let mid = mid_width(sentence.as_ref().map(|(t, _)| t.as_str()), &answers, &caps);
    #[allow(clippy::cast_precision_loss)]
    let arrangement = baylee_client_core::ledge::arrange(
        window_w as f32,
        baylee_client_core::ledge::Columns {
            left: LEFT_RESERVED,
            mid,
            right: RIGHT_RESERVED,
        },
        caps_w,
    );

    let columns = [
        column_node(Side::Left),
        column_node(Side::Mid(arrangement.mid_x, window_w)),
        column_node(Side::Right),
    ]
    .map(|node| commands.spawn(node).id());
    commands.entity(shelf).add_children(&columns);

    // Left (the mana pool) and right (a draw offer and a concession) arrive
    // in their own steps; their room is reserved above so that nothing the
    // middle does moves when they land.
    let middle = columns[1];

    // `Split` is the rung that sends the sentence into the drawer, and there
    // is no drawer yet. Until there is, it draws what `Compact` draws: a
    // question the player has already read is worth less than the buttons,
    // which is exactly why `Split` gives it up — but dropping it on the floor
    // instead of putting it somewhere is not the same trade.
    let shows_sentence = arrangement.density.shows_sentence()
        || arrangement.density == baylee_client_core::ledge::Density::Split;
    if let Some((text, alarming)) = sentence.filter(|_| shows_sentence) {
        let ink = if alarming {
            palette::DANGER
        } else if waiting {
            palette::DIALOG_SOFT
        } else {
            palette::DIALOG_INK
        };
        let line = self::sentence(&mut commands, &fonts, &text, ink);
        commands.entity(middle).add_child(line);
    }

    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(baylee_client_core::ledge::BUTTON_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(middle).add_child(row);

    if let Some(words) = armed {
        // The armed card replaces the answers rather than joining them: the
        // first button *is* the answer, and the second is the way back.
        armed_row(&mut commands, &fonts, lang, &prefs, row, &words);
    } else {
        for (i, (action, label)) in answers.iter().enumerate() {
            let cap = if arrangement.density.shows_keycaps() {
                caps[i].as_deref()
            } else {
                None
            };
            let weight = if i == 0 {
                Weight::Candle
            } else {
                Weight::Secondary
            };
            let button = answer(&mut commands, &fonts, label, weight, cap);
            commands
                .entity(button)
                .insert(PromptButton { action: *action });
            commands.entity(row).add_child(button);
        }
    }
}

/// Which answers this question takes, in the order they are offered.
///
/// Lifted out of `sync_overlay` unchanged, including the two suppressions
/// that are easy to read as bugs. `cast_menu` takes the answers away because
/// the engine's window *behind* the client's own chooser is an ordinary
/// priority, and "Pass priority" under "Choose how it is cast" is two primary
/// answers saying opposite things. `elsewhere` takes the Confirm away because
/// the zone browser's footer already draws one, and a player ticking a
/// fetchland's target saw the same word twice on one screen.
fn answers_for(
    duel: &Duel,
    lang: Lang,
    over: bool,
    waiting: bool,
    elsewhere: bool,
) -> Vec<(PromptAction, String)> {
    use baylee_engine::choice::Pending;
    if over || waiting || duel.cast_menu.is_some() {
        return Vec::new();
    }
    let say = |action: PromptAction, phrase: Phrase| (action, phrase.text(lang).to_string());
    match duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::pending)
    {
        Some(Pending::Mulligan { .. }) => vec![
            say(PromptAction::Keep, Phrase::KeepHand),
            say(PromptAction::Mulligan, Phrase::TakeMulligan),
        ],
        Some(Pending::YesNo { .. }) => vec![
            say(PromptAction::Yes, Phrase::ActAnswerYes),
            say(PromptAction::No, Phrase::ActAnswerNo),
        ],
        // Priority is not confirmed, it is *passed*, and the two words are
        // not interchangeable on a button: "OK" acknowledges something that
        // has already happened. "Skip turn" beside it is the same decision at
        // the larger size, and is where the phase rail's fast-forward went.
        Some(Pending::Priority { .. }) => vec![
            say(PromptAction::Confirm, Phrase::PassPriority),
            say(PromptAction::SkipTurn, Phrase::SkipTheTurn),
        ],
        // Combat always offers all three, including with nothing declared:
        // "none" is a real answer, and the step does not end without one.
        Some(Pending::ChooseAttackers { .. }) => vec![
            say(PromptAction::AimNext, Phrase::AimNext),
            say(PromptAction::Confirm, Phrase::Attack),
            say(PromptAction::DeclareNothing, Phrase::DeclareNone),
        ],
        Some(Pending::ChooseBlockers { .. }) => vec![
            say(PromptAction::AimNext, Phrase::AimNext),
            say(PromptAction::Confirm, Phrase::Block),
            say(PromptAction::DeclareNothing, Phrase::DeclareNone),
        ],
        Some(_)
            if !elsewhere
                && duel
                    .interaction
                    .as_ref()
                    .is_some_and(baylee_client_core::Interaction::can_confirm) =>
        {
            vec![say(PromptAction::Confirm, Phrase::ConfirmOk)]
        }
        _ => Vec::new(),
    }
}

/// The legend on each answer's keycap, or `None` where the answer has no key.
///
/// **Always out of the keymap**, never out of a string in the code: a player
/// who rebinds `Space` sees the new chord on the next frame.
/// [`baylee_client_core::ledge::shortcut_for`] is the bridge from an answer to
/// the action that sends it, and `chords` is the account's own binding of
/// that action. `first` and not `[0]`, because an action a player has unbound
/// is an answer with no cap rather than a panic.
fn keys_for(
    prefs: &crate::prefs::Prefs,
    answers: &[(PromptAction, String)],
    armed: bool,
) -> Vec<Option<String>> {
    let legend = |action| {
        prefs
            .keymap()
            .chords(action)
            .first()
            .map(baylee_client_core::prefs::Chord::display)
    };
    if armed {
        // Not a `PromptAction` between them: an armed deed is the client's
        // own two-stage commit, fired by `Action::Primary` and taken back by
        // `Action::Cancel` (`input::armed_keys`), so the bridge does not
        // reach it and these two are named directly.
        return vec![
            legend(baylee_client_core::prefs::Action::Primary),
            legend(baylee_client_core::prefs::Action::Cancel),
        ];
    }
    answers
        .iter()
        .map(|(action, _)| baylee_client_core::ledge::shortcut_for(*action).and_then(legend))
        .collect()
}

/// Which of the three columns a node is, and where its centre goes.
enum Side {
    Left,
    /// The middle, with the centre [`baylee_client_core::ledge::arrange`] put
    /// it on and the width of the window it was measured against.
    Mid(f32, i32),
    Right,
}

/// One column, standing in the shelf's own band.
///
/// The vertical inset is written out rather than inherited, because the
/// shelf's padding does not reach an absolutely positioned child: taffy
/// measures an inset against the border box less the *border*, so the band
/// below the lip is `LEDGE_PAD_Y - LIP` down from the top and `LEDGE_PAD_Y`
/// up from the bottom, which is exactly [`BUTTON_H`] of room.
fn column_node(side: Side) -> impl Bundle {
    let mut node = Node {
        position_type: PositionType::Absolute,
        top: px(LEDGE_PAD_Y - LIP),
        bottom: px(LEDGE_PAD_Y),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(baylee_client_core::ledge::SENTENCE_GAP),
        ..default()
    };
    match side {
        Side::Left => {
            node.left = px(EDGE);
            node.justify_content = JustifyContent::Start;
        }
        Side::Right => {
            node.right = px(EDGE);
            node.justify_content = JustifyContent::End;
        }
        // Full width and centred, so the question stands on the **window's**
        // middle — which is the middle of this seat's own mat. When `arrange`
        // has had to slide it off centre, the slide is paid for out of one
        // side's padding: a centred child in a box padded by `p` on the left
        // has its centre at `(p + w) / 2`, so `p = 2 · shift`.
        Side::Mid(mid_x, window_w) => {
            node.left = px(0);
            node.right = px(0);
            node.justify_content = JustifyContent::Center;
            #[allow(clippy::cast_precision_loss)]
            let shift = 2.0 * (mid_x - window_w as f32 / 2.0);
            node.padding = if shift >= 0.0 {
                UiRect::left(px(shift))
            } else {
                UiRect::right(px(-shift))
            };
        }
    }
    // The middle lies over the other two across the whole width, so without
    // this a draw offer and a concession would be dead or alive depending on
    // the order the three were spawned in — "a label swallows the hover", one
    // level up. The buttons inside it are pickable in their own right.
    (node, Pickable::IGNORE)
}

/// The shelf's prose: the question, or whatever has replaced it.
///
/// The slip's voice on the dialog's ground. The slant is what carried the
/// question on parchment and it carries it here — a question is *written*,
/// where a button is stamped — but nothing else of the sheet comes with it:
/// no bleed, which is what a nib does to fibres, and no parchment ink.
/// Bracketed asides go grey through the same
/// [`baylee_client_core::prose::bracketed`] the slip used, because a key to
/// press or a count the board already shows is not part of the sentence.
fn sentence(commands: &mut Commands, fonts: &UiFonts, text: &str, ink: Color) -> Entity {
    let line = commands
        .spawn((
            Text::default(),
            tf_italic(fonts, SENTENCE_PT),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    for (run, aside) in baylee_client_core::prose::bracketed(text) {
        let span = commands
            .spawn((
                TextSpan::new(run.to_string()),
                tf_italic(fonts, SENTENCE_PT),
                TextColor(if aside { palette::DIALOG_SOFT } else { ink }),
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    line
}

/// How loud an answer is.
///
/// Three and not five: **Danger** (a concession waiting for its second press)
/// and **Dead** (a draw that cannot be offered) belong to the right column
/// and arrive with it, along with the one new colour they need. A weight with
/// no caller is a weight nothing has ever drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Weight {
    /// The answer the engine is asking for — pass, attack, keep, yes, OK.
    ///
    /// Exactly one per question, and it is the **marking of the default
    /// button**: the one answer `Space` sends is the one that burns. Rejected:
    /// brass, which is a light on a *card* and means a thing already taken;
    /// and an underline as well, which is a second mark for one claim.
    Candle,
    /// Every other answer to the same question.
    Secondary,
    /// A way back out rather than an answer: cancel, ask again.
    Ghost,
}

impl Weight {
    /// Fill, border, ink.
    const fn colours(self) -> (Color, Color, Color) {
        match self {
            // `DIALOG` on `CANDLE` is 7.39 : 1. Light ink on the candle is
            // 1.86 : 1 and is forbidden outright — see the palette.
            Self::Candle => (palette::CANDLE, palette::CANDLE, palette::DIALOG),
            // The border is not decoration: `DIALOG_LIT` on `DIALOG` is
            // 1.10 : 1, so a secondary answer without one is invisible.
            Self::Secondary => (
                palette::DIALOG_LIT,
                palette::DIALOG_LINE,
                palette::DIALOG_INK,
            ),
            Self::Ghost => (Color::NONE, Color::NONE, palette::DIALOG_INK),
        }
    }

    /// How it answers the pointer.
    ///
    /// `Feel::new` shades a colour towards white and **keeps its alpha**, so
    /// a ghost resting at `Color::NONE` would be lifted to a brighter nothing
    /// and never answer the pointer at all. Its hot end is therefore stated.
    fn feel(self) -> Feel {
        match self {
            Self::Candle => Feel::new(palette::CANDLE),
            Self::Secondary => Feel::new(palette::DIALOG_LIT),
            Self::Ghost => Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
        }
    }

    /// The cap's own fill, border and legend, which are not the button's.
    ///
    /// A keycap is a dark key **on** the answer, whatever the answer is, so
    /// the fill is the shelf's own ground in all three cases. What differs is
    /// the legend: `CANDLE` on `DIALOG` reads at 7.39 : 1 where `DIALOG_SOFT`
    /// on the same ground reads at 4.69 : 1, and the brighter of the two
    /// belongs on the button the question is asking for.
    ///
    /// A ghost's cap is the secondary's exactly. `DIALOG_LIT` as its fill
    /// would measure 4.28 : 1 against the legend and fall under 4.5.
    const fn cap_colours(self) -> (Color, Color, Color) {
        match self {
            Self::Candle => (palette::DIALOG, palette::DIALOG, palette::CANDLE),
            Self::Secondary | Self::Ghost => {
                (palette::DIALOG, palette::DIALOG_LINE, palette::DIALOG_SOFT)
            }
        }
    }
}

/// One answer on the shelf: a keycap, and what pressing it does.
///
/// A sibling of `overlay::answer_button` rather than a change to it, and
/// deliberately: that one draws on **parchment**, and the end screen and the
/// lobby's own `Press` still stand on paper. Two registers, two functions,
/// one shape.
///
/// The label carries `Pickable::IGNORE` for the reason every label in this
/// client does: a `Text` is a `Node`, so a pickable one sits in front of the
/// button and `Feel` animates the padding while the middle goes dead.
pub(super) fn answer(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    weight: Weight,
    cap: Option<&str>,
) -> Entity {
    let (fill, edge, ink) = weight.colours();
    let button = commands
        .spawn((
            Node {
                height: px(BUTTON_H),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(CAP_GAP),
                padding: UiRect::axes(px(BUTTON_PAD_X), px(BUTTON_PAD_Y)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            weight.feel(),
        ))
        .id();
    if let Some(legend) = cap {
        let (cap_fill, cap_edge, cap_ink) = weight.cap_colours();
        let key = keycap(commands, fonts, legend, cap_fill, cap_ink, cap_edge, CAP_PT);
        commands.entity(button).add_child(key);
    }
    let words = commands
        .spawn((
            Text::new(label.to_string()),
            tf_bold(fonts, LABEL_PT),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(words);
    button
}

/// The armed deed as a pair of buttons: the deed itself, and the way back.
///
/// Two buttons and no sentence between them, because the first one *is* the
/// sentence — a line reading "Play this card" beside a button called "Send"
/// says the same thing twice and leaves a player to work out which half is
/// the button. It is the one state that takes the question off the shelf.
fn armed_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    prefs: &crate::prefs::Prefs,
    row: Entity,
    words: &super::overlay::ArmedWords,
) {
    let caps = keys_for(prefs, &[], true);
    let cancel = super::overlay::ArmedWords {
        text: Phrase::ArmedCancel.text(lang).to_string(),
        cost: None,
    };
    for (i, (action, words, weight)) in [
        (MenuAction::SendArmed, words, Weight::Candle),
        (MenuAction::CancelArmed, &cancel, Weight::Ghost),
    ]
    .into_iter()
    .enumerate()
    {
        let (_, _, ink) = weight.colours();
        let button = answer(commands, fonts, "", weight, caps[i].as_deref());
        commands.entity(button).insert(MenuButton { action });
        // The phrase splits at its `{0}`; one with none, or a deed with no
        // price to quote, is one piece and the pips are skipped. A cost is
        // **drawn** and never spelled — `{4}{U}{U}` as letters is the thing
        // the deck builder deliberately does not do.
        let (head, tail) = words
            .cost
            .and_then(|_| words.text.split_once("{0}"))
            .unwrap_or((words.text.as_str(), ""));
        put_words(commands, fonts, button, head.trim(), ink);
        if let Some(cost) = words.cost {
            for pip in baylee_client_core::manapip::cost(&cost) {
                let mark = crate::manaui::spawn_pip(commands, fonts, pip, 15.0);
                commands.entity(button).add_child(mark);
            }
        }
        put_words(commands, fonts, button, tail.trim(), ink);
        commands.entity(row).add_child(button);
    }
}

/// One piece of a button's words, or nothing at all when the piece is empty.
fn put_words(commands: &mut Commands, fonts: &UiFonts, button: Entity, text: &str, ink: Color) {
    if text.is_empty() {
        return;
    }
    let node = crate::manaui::spawn_rich_label(commands, fonts, text, LABEL_PT, ink);
    commands.entity(button).add_child(node);
}

/// How wide a keycap's box is, legend and air together.
///
/// [`KEYCAP_SIDE`] is a floor and not a width: a cap grows with its legend,
/// so `F6` sits in the 20-pixel square and `⇧Tab` does not.
fn cap_width(legend: &str) -> f32 {
    (CAP_PT * KEYCAP_SIDE).max(super::text_width(legend, CAP_PT, true) + 2.0 * CAP_PT * 0.45)
}

/// How wide the middle column will be, before it is laid out.
///
/// The estimate [`baylee_client_core::ledge::arrange`] is fed, and the reason
/// [`super::text_width`] exists — `bevy_ui` measures text during layout, and
/// this is a decision the layout depends on.
fn mid_width(
    sentence: Option<&str>,
    answers: &[(PromptAction, String)],
    caps: &[Option<String>],
) -> f32 {
    let buttons: f32 = answers
        .iter()
        .enumerate()
        .map(|(i, (_, label))| {
            let cap = caps
                .get(i)
                .and_then(Option::as_deref)
                .map_or(0.0, |c| cap_width(c) + CAP_GAP);
            cap + super::text_width(label, LABEL_PT, true) + 2.0 * BUTTON_PAD_X
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let gaps = baylee_client_core::ledge::BUTTON_GAP * answers.len().saturating_sub(1) as f32;
    let words = sentence.map_or(0.0, |text| {
        super::text_width(text, SENTENCE_PT, false) + baylee_client_core::ledge::SENTENCE_GAP
    });
    words + buttons + gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shelf is a **dialog**, and nothing on it is borrowed from the
    /// parchment or from the table.
    ///
    /// Its own scan rather than an addition to `sheet.rs`'s, which is bound
    /// to `sheet.rs` and `overlay.rs` by name and would never have looked at
    /// a new file. Three things are forbidden and each for its own reason:
    ///
    /// - `palette::ACCENT` is the teal `docs/design.md` §1.2 retires. Candle
    ///   is what replaces it, and a shelf that kept one teal control would be
    ///   the retirement half-done in the most visible place there is.
    /// - `BRASS` as a **letter** is gilt, which on this client means a thing
    ///   already *taken* — an armed deed, a place in an ordering. The shelf
    ///   asks; it does not report. (As a fill it never appears here at all,
    ///   so the needle is the `TextColor`.)
    /// - `PARCHMENT` in any form is the sheet a question was written on, and
    ///   §3.1 is the whole argument for why it is not written on one now.
    #[test]
    fn the_ledge_speaks_only_dialog() {
        for forbidden in [
            "palette::ACCENT",
            "TextColor(palette::BRASS",
            "palette::PARCHMENT",
        ] {
            assert!(
                !drawn().contains(forbidden),
                "`{forbidden}` is on the shelf, which is a dialog: the ledge \
                 has one register and this is not in it"
            );
        }
    }

    /// The half of this file that draws, which is what a scan is about.
    ///
    /// Everything below `#[cfg(test)]` is these tests, and the needles they
    /// name are written out here in full — a scan over the whole file finds
    /// its own list and reports the rule as the violation. Cutting at the
    /// attribute is the shortest honest answer; the alternative is counting
    /// occurrences, which passes the moment a second one appears in a doc
    /// comment.
    ///
    /// The drawer joins this when it exists (§10.2 step 6): it is the same
    /// surface with the same rules, and a scan that read only half of it
    /// would be `sheet.rs`'s file-bound scan made twice.
    fn drawn() -> &'static str {
        include_str!("ledge.rs")
            .split_once("#[cfg(test)]")
            .expect("the tests are still where they were")
            .0
    }

    /// What an answer is written in has to be readable on what it is written
    /// on.
    ///
    /// The pair the shelf lives or dies by: `DIALOG` on `CANDLE` is the ink
    /// of every default button, and the *inverse* — light ink on the candle —
    /// is 1.86 : 1 and is forbidden outright by §3.2. A test on the ratio
    /// alone would pass on either, so both ends are stated.
    ///
    /// `LEDGE_DEAD / DIALOG ≥ 3.0` is the third pair §3.2 names and it is not
    /// here yet: the colour arrives with the disabled draw button in §10.2
    /// step 5, and a constant whose only reader is a test is a constant
    /// nothing draws.
    #[test]
    fn a_candle_is_dark_enough_to_write_on() {
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

        let ink = contrast(palette::DIALOG, palette::CANDLE);
        assert!(
            ink >= 4.5,
            "the candle's own label has to be readable: {ink:.2}:1"
        );
        let wrong = contrast(palette::DIALOG_INK, palette::CANDLE);
        assert!(
            wrong < 3.0,
            "this test's premise is that light ink on a candle cannot be \
             read, and it measured {wrong:.2}:1"
        );
        // The keycap on a secondary answer: a dark key on the shelf's own
        // ground, and the quiet legend still has to carry 11-point text.
        let legend = contrast(palette::DIALOG_SOFT, palette::DIALOG);
        assert!(
            legend >= 4.5,
            "a keycap nobody can read is a key nobody presses: {legend:.2}:1"
        );
    }

    /// The shelf does not follow the pointer, and the struct is where that is
    /// decided.
    ///
    /// [`LedgeRevision`] exists *because* [`super::HudRevision`] counts the
    /// hover; a `hovered` field here would put the shelf back in the rebuild
    /// it was taken out of, and every `Feel` on it back to rest whenever the
    /// pointer crossed a hand card. Nothing about that is visible at the call
    /// site — the field would simply be compared and assigned like any other
    /// — so it is read out of the source, the way `hud/tests.rs` reads
    /// `HudRevision`'s own fields.
    #[test]
    fn the_shelf_does_not_follow_the_pointer() {
        let source = include_str!("ledge.rs");
        let body = source
            .split_once("pub struct LedgeRevision {")
            .expect("the struct is still called that")
            .1;
        let body = body.split_once("\n}").expect("and still closes").0;
        let fields: Vec<&str> = body
            .lines()
            .filter_map(|line| {
                let name = line.trim().strip_prefix("pub(super) ")?;
                let (name, _) = name.split_once(':')?;
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    .then_some(name)
            })
            .collect();
        assert!(fields.len() > 8, "the fields did not parse: {fields:?}");
        for field in fields {
            assert!(
                !field.contains("hover"),
                "`{field}` puts the shelf back in the hover's rebuild, which \
                 is the one thing this counter exists to keep it out of"
            );
        }
    }

    /// A question that has to give something up gives up its keycaps before
    /// its words, and its words before its buttons.
    ///
    /// The renderer's half of `client-core`'s ladder: `arrange` decides the
    /// rung and this is what the rung is spent on. `Split` is the rung that
    /// sends the sentence to the drawer, and until there is a drawer it draws
    /// what `Compact` draws — a question the player has already read is worth
    /// less than the buttons, which is why `Split` gives it up, but dropping
    /// it on the floor instead of putting it somewhere is a different trade.
    #[test]
    fn the_rungs_are_spent_on_the_caps_first() {
        use baylee_client_core::ledge::Density;
        assert!(Density::Full.shows_keycaps() && Density::Full.shows_sentence());
        assert!(!Density::Compact.shows_keycaps() && Density::Compact.shows_sentence());
        assert!(!Density::Split.shows_keycaps() && !Density::Split.shows_sentence());
        // And the one deviation, stated where it can be found again.
        assert!(
            include_str!("ledge.rs").contains("|| arrangement.density == "),
            "`Split` no longer borrows `Compact`'s sentence: either the \
             drawer exists and this line should go, or the question is being \
             dropped"
        );
    }

    /// A keycap is a floor and not a width.
    ///
    /// `F6` sits in the 20-pixel square [`KEYCAP_SIDE`] gives it and `⇧Tab`
    /// does not, which is the whole reason `sheet::cap`'s box grows with its
    /// legend — and the reason §2.3's own worked example is 18 pixels light:
    /// it measures `[Space]` at the square, and "Space" is five characters.
    #[test]
    fn a_long_chord_gets_a_wider_key() {
        let square = CAP_PT * KEYCAP_SIDE;
        assert!(
            (cap_width("6") - square).abs() < f32::EPSILON,
            "one character sits in the square: {}",
            cap_width("6")
        );
        for chord in ["\u{21e7}Tab", "Space"] {
            assert!(
                cap_width(chord) > square,
                "`{chord}` does not, and clipping a chord would be worse than \
                 growing its key: {}",
                cap_width(chord)
            );
        }
        // And the slip in §2.3's own worked example, written down where it
        // can be checked: it measures `[Space]` at the square, which is 18
        // pixels light. The priority middle is 606 and not 588 — still
        // `Full` at 1280 against 325 and 214, so nothing downstream moves.
        assert!(
            cap_width("Space") - square > 17.0,
            "the design's arithmetic was out by {}",
            cap_width("Space") - square
        );
    }
}
