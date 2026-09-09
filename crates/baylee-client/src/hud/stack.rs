//! The stack, drawn as cards.
//!
//! Each entry is the spell's own picture — or, for an ability, the picture
//! of the permanent it came from — followed by a row of everything it
//! targets, each drawn as its own smaller card.
//!
//! # One sentence, then a queue
//!
//! The panel does not draw its entries as peers. The next thing to resolve is
//! a **full** row — a card an inch across, the spell's name at reading size,
//! an arrow and a picture of everything it points at — and everything under
//! it is a **compact** row: a smaller card, one line of name, smaller
//! thumbnails and no arrow. That is the whole hierarchy, and it is one
//! decision rather than two, because the size ramp *is* the depth cue.
//!
//! The arithmetic forces it. A full row is [`STACK_CARD_H`] plus its padding,
//! about 104 px; the panel is 62% of the window, which on a laptop is a
//! little over six hundred. Six uniform rows and the seventh is clipped with
//! nothing to say it was — and a stack of ten is a perfectly ordinary
//! storm turn. One full row and [`STACK_COMPACT_ROWS`] compact ones fit in
//! the same space, and what still does not fit is *counted* on a last line
//! rather than silently cut off.
//!
//! # Arriving
//!
//! A row eases in: it lifts into place, grows the last few percent, and its
//! ink and its fills come up from nothing. The progress cannot live in the
//! row, because the HUD is a retained tree rebuilt whenever [`HudRevision`]
//! changes and *hover* is part of that gate — a pointer twitch during the
//! quarter second an arrival takes would despawn the row and spawn it again,
//! and a fade that restarts under the pointer reads as a flicker. It lives in
//! [`StackMotion`] instead, keyed by what the row draws, so a rebuild
//! re-attaches to the arrival already in progress.
//!
//! [`HudRevision`]: super::HudRevision

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The card a stack entry is drawn at, at the top of the panel.
const STACK_CARD_W: f32 = 66.0;
/// Height of that card.
const STACK_CARD_H: f32 = STACK_CARD_W * 88.0 / 63.0;
/// The card a *queued* entry is drawn at — two thirds of the top one, which
/// is the whole of the ordering cue.
const STACK_QUEUED_W: f32 = 44.0;
/// Height of that card.
const STACK_QUEUED_H: f32 = STACK_QUEUED_W * 88.0 / 63.0;
/// The smaller card a *target* is drawn at, so the two never read as peers:
/// the thing on the stack is the sentence, its targets are its objects.
const STACK_TARGET_W: f32 = 38.0;
/// Height of a target thumbnail.
const STACK_TARGET_H: f32 = STACK_TARGET_W * 88.0 / 63.0;
/// A target of a queued entry, at the same two thirds.
const STACK_QUEUED_TARGET_W: f32 = 25.0;
/// Height of that thumbnail.
const STACK_QUEUED_TARGET_H: f32 = STACK_QUEUED_TARGET_W * 88.0 / 63.0;
/// Panel width. Wide enough for a card, an arrow and three targets.
const STACK_PANEL_W: f32 = 296.0;

/// How many compact rows are drawn under the full one.
///
/// The panel is `max_height: 62%`; on the 1052-logical-pixel window this
/// client is developed against that is 652, less 20 of padding and 34 of
/// title leaves 598. A full row is 104 and a compact one about 70, both plus
/// the 6 px gap: `104 + 6 + 6 × 76 = 566`, and the seventh compact row would
/// be the first to be cut. Anything past that is counted on one line
/// instead — a number is a worse drawing than a card and a much better one
/// than a silent clip.
const STACK_COMPACT_ROWS: usize = 6;

/// How fast a row arrives, per second.
///
/// The same exponential everything in this client eases with, at a rate that
/// puts a row at about nine tenths after a fifth of a second. Deliberately
/// slower than [`crate::ambience::FEEL_RATE`]: that one answers a pointer,
/// where any delay is lag, and this one announces an event the player did not
/// necessarily cause.
const ARRIVE_RATE: f32 = 13.0;

/// How far above its resting place an entry starts, in logical pixels.
///
/// Up rather than down, because the stack grows downwards on the screen and a
/// new object lands on *top* of it: a row rising into the gap it just made
/// says which end of the queue it joined.
const ARRIVE_LIFT: f32 = 14.0;

/// The same for the panel, which slides rather than grows.
const PANEL_LIFT: f32 = 10.0;

/// The scale an arriving row starts at.
///
/// Small enough to read as a movement towards the reader, far enough from
/// zero that the row is never a dot: a card that scaled from nothing would
/// be an effect, and this is a notification.
const ARRIVE_SCALE: f32 = 0.96;

/// Which part of the panel a node's arrival is remembered under.
///
/// The *shape* is part of the key on purpose. A row promoted from compact to
/// full — which is exactly what a resolution looks like from the panel's side
/// — is a different drawing of the same object, and a fresh key is what makes
/// the promotion ease rather than cut. It also means departure needs no
/// animation at all: the object that resolved is gone from the view, drawing
/// a ghost of it would be drawing something the view no longer carries, and
/// the *visible* event is the new top rising into the full row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackKey {
    /// The panel itself, which arrives when the stack stops being empty.
    Panel,
    /// One entry, and whether it is drawn as the full row.
    Entry(ObjectId, bool),
}

/// How far each row of the stack panel has arrived, `0.0`…`1.0`.
///
/// A `Vec` rather than a map: a stack with more than a dozen objects on it is
/// a rules oddity rather than a case to optimise for, and a linear scan over
/// eight pairs is cheaper than hashing one.
#[derive(Resource, Default)]
pub struct StackMotion {
    /// One pair per row on screen, in no particular order.
    rows: Vec<(StackKey, f32)>,
}

impl StackMotion {
    /// How far `key` has arrived; `1.0` for anything not being tracked, so a
    /// node whose row has finished is simply drawn.
    fn progress(&self, key: StackKey) -> f32 {
        self.rows
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(1.0, |(_, p)| *p)
    }
}

/// Which of a node's colours the arrival is written to.
///
/// Load-bearing rather than tidy. [`Node`] *requires* a [`BackgroundColor`]
/// and a [`BorderColor`], so every node in this panel carries all three
/// colours whether it draws them or not — a line of text has a background of
/// [`Color::NONE`]. A system that simply wrote whichever colour it found
/// would take that transparent black, multiply its alpha back up and give
/// every label in the panel an opaque plate. The first live shot of this
/// panel is exactly that: eight black rectangles where the words should be.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Paints {
    /// A line of text: its [`TextColor`] and nothing else.
    Ink,
    /// A fill: its [`BackgroundColor`].
    Fill,
    /// A fill that runs the other way — opaque while the row arrives and gone
    /// once it has landed. It is how the card picture fades: the art is a
    /// [`MaterialNode`] on a material shared with every card that looks the
    /// same (`a_look_is_shared_by_exactly_what_looks_the_same`), so an alpha
    /// written there would fade the player's whole hand with it.
    Veil,
}

/// A node that eases in with the row it belongs to.
///
/// Every piece of a row carries one — the row's own fill, each line of text,
/// each chip — because [`bevy_ui`] has no opacity inheritance: fading a
/// subtree means writing an alpha per entity, and an entity that knows its
/// own row needs no tree walk to be found.
#[derive(Component)]
pub struct Arriving {
    /// The row whose progress this node follows.
    key: StackKey,
    /// The alpha the node is drawn at once it has arrived.
    base: f32,
    /// Which colour that alpha belongs to.
    paints: Paints,
}

impl Arriving {
    /// Text that fades up to the alpha of the colour it is set in.
    const fn ink(key: StackKey, base: f32) -> Self {
        Self {
            key,
            base,
            paints: Paints::Ink,
        }
    }

    /// A fill that fades up to the alpha of the colour it is drawn in.
    const fn fill(key: StackKey, base: f32) -> Self {
        Self {
            key,
            base,
            paints: Paints::Fill,
        }
    }

    /// A veil over a card picture, opaque until the row has landed.
    const fn veil(key: StackKey) -> Self {
        Self {
            key,
            base: 1.0,
            paints: Paints::Veil,
        }
    }

    /// The alpha this node should be drawn at, `progress` of the way in.
    fn alpha(&self, progress: f32) -> f32 {
        self.base
            * if self.paints == Paints::Veil {
                1.0 - progress
            } else {
                progress
            }
    }
}

/// The row (or the panel) itself, which also lifts and grows into place.
#[derive(Component)]
pub struct ArrivingRow {
    /// How far above its resting place it starts, in logical pixels.
    lift: f32,
    /// The scale it starts at; `1.0` for something that only slides.
    from: f32,
    /// The border it rests at, so the accent rail on the top row arrives with
    /// the row rather than standing there alone while the row fades in behind
    /// it. Kept here rather than read back off the node, because the resting
    /// colour of a row with no rail is [`Color::NONE`] and reading *that* back
    /// as a base is the black-plate trap one component up.
    rail: Color,
}

/// Eases every arriving row towards its resting state.
///
/// Runs after [`sync_overlay`] and deliberately: a row spawned this frame is
/// spawned at *rest*, and without the ordering it would be drawn once at full
/// strength before this system ever saw it — a one-frame flash at the top of
/// the panel, which is the one place in the interface a player is watching.
///
/// [`sync_overlay`]: super::sync_overlay
pub fn ease_the_stack_in(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut motion: ResMut<StackMotion>,
    mut fades: Query<(
        &Arriving,
        Option<&mut TextColor>,
        Option<&mut BackgroundColor>,
    )>,
    mut rows: Query<(&Arriving, &ArrivingRow, &mut UiTransform, &mut BorderColor)>,
) {
    // Which rows are on screen. Anything else has resolved, or the panel has
    // gone with the last object on it, and its progress goes with it — an id
    // kept after its row left would greet the *next* spell of that name with
    // no animation at all.
    //
    // `fades` is the wider query — a row carries an [`Arriving`] too — so it
    // is the one that sees every key.
    let mut live: Vec<StackKey> = Vec::new();
    for (arriving, ..) in &fades {
        if !live.contains(&arriving.key) {
            live.push(arriving.key);
        }
    }
    // A row that steps *down* is not an arrival. When a spell lands on a
    // stack that already had one, yesterday's top is drawn queued from this
    // frame on — a different key for the same object, which would otherwise
    // be seeded at nothing and fade in beside the newcomer, so two rows would
    // announce themselves and only one of them would be new. Carrying the
    // progress across the key change leaves it standing where it was.
    //
    // The other direction is deliberately not carried: a queued row becoming
    // full is what a resolution looks like from the panel, and that is the
    // movement the player is meant to see.
    for &key in &live {
        let StackKey::Entry(id, false) = key else {
            continue;
        };
        let known = motion.rows.iter().any(|(k, _)| *k == key);
        let stepped_down = motion
            .rows
            .iter()
            .any(|(k, _)| *k == StackKey::Entry(id, true));
        if !known && stepped_down {
            motion.rows.push((key, 1.0));
        }
    }

    motion.rows.retain(|(key, _)| live.contains(key));
    for key in live {
        if !motion.rows.iter().any(|(k, _)| *k == key) {
            motion.rows.push((key, 0.0));
        }
    }

    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let step = if still {
        1.0
    } else {
        1.0 - (-ARRIVE_RATE * time.delta_secs()).exp()
    };
    let mut moving = false;
    for (_, progress) in &mut motion.rows {
        if *progress >= 1.0 {
            continue;
        }
        *progress = (*progress + (1.0 - *progress) * step).min(1.0);
        if *progress > 0.999 {
            *progress = 1.0;
        }
        moving = true;
    }
    // Every node is spawned at rest, so a panel that has finished arriving
    // needs nothing written to it at all — including on the frame it is
    // rebuilt under the pointer.
    if !moving {
        return;
    }

    for (arriving, ink, fill) in &mut fades {
        let alpha = arriving.alpha(motion.progress(arriving.key));
        match arriving.paints {
            Paints::Ink => {
                if let Some(mut ink) = ink {
                    ink.0 = ink.0.with_alpha(alpha);
                }
            }
            Paints::Fill | Paints::Veil => {
                if let Some(mut fill) = fill {
                    fill.0 = fill.0.with_alpha(alpha);
                }
            }
        }
    }
    for (arriving, row, mut transform, mut border) in &mut rows {
        let progress = motion.progress(arriving.key);
        transform.translation.y = px(-row.lift * (1.0 - progress));
        transform.scale = Vec2::splat(row.from + (1.0 - row.from) * progress);
        *border = BorderColor::all(row.rail.with_alpha(row.rail.alpha() * progress));
    }
}

/// The stack, next-to-resolve at the top; pinned left of the phase rail.
///
/// Drawn as cards rather than as a list of names, because the stack is the one
/// place in the game where "what is about to happen, and to what" has to be
/// read in a hurry — and a player who has to match two names against a board
/// of twelve permanents is doing the engine's bookkeeping by hand. Each entry
/// is the spell (or the permanent whose ability it is), an arrow, and a
/// picture of everything it points at.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the material store
#[allow(clippy::too_many_lines)] // a title, a queue and a tally, built flat
pub(super) fn spawn_stack_panel(
    commands: &mut Commands,
    lang: Lang,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let key = StackKey::Panel;
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                top: px(TAB_H + RAIL_H + 12.0),
                width: px(STACK_PANEL_W),
                max_height: percent(62),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            ZIndex(1),
            upward_shadow(),
            Pickable::IGNORE,
            Arriving::fill(key, palette::PANEL.alpha()),
            ArrivingRow {
                lift: PANEL_LIFT,
                from: 1.0,
                rail: Color::NONE,
            },
        ))
        .id();

    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(head);

    let title = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
            children![
                (
                    Text::new(Phrase::StackTitle.text(lang)),
                    tf(fonts, 13.0),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                ),
                (
                    Node {
                        padding: UiRect::axes(px(6.0), px(1.0)),
                        border_radius: BorderRadius::all(px(8)),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    Arriving::fill(key, palette::PANEL_LIT.alpha()),
                    children![(
                        Text::new(board.stack.len().to_string()),
                        tf(fonts, 13.0),
                        TextColor(palette::INK),
                        Arriving::ink(key, palette::INK.alpha()),
                    )],
                ),
            ],
        ))
        .id();
    commands.entity(head).add_child(title);

    // Whose answer the table is waiting for. It is the one thing about the
    // stack the prompt slip cannot say — the slip speaks to *this* seat, and
    // between two of an opponent's spells there is nothing on it at all —
    // and it costs one lookup. Nothing while the stack is resolving, which is
    // when nobody holds priority.
    if let Some(holder) = view.priority {
        let waiting = commands
            .spawn((
                Text::new(Phrase::WaitingFor.fill(lang, &[statics.seat_name(holder)])),
                tf(fonts, 11.0),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(head).add_child(waiting);
    }

    let shown = board.stack.len().min(1 + STACK_COMPACT_ROWS);
    for item in board.stack.iter().take(shown) {
        let entry = spawn_stack_entry(
            commands,
            lang,
            item,
            item.depth == 0,
            view,
            statics,
            textures,
            assets,
            fonts,
            faces,
            cards.as_deref_mut(),
        );
        commands.entity(panel).add_child(entry);
    }

    if let Some(hidden) = board.stack.len().checked_sub(shown).filter(|n| *n > 0) {
        let more = commands
            .spawn((
                Node {
                    margin: UiRect::top(px(2)),
                    ..default()
                },
                Pickable::IGNORE,
                children![(
                    Text::new(Phrase::StackMore.fill(lang, &[&hidden.to_string()])),
                    tf(fonts, 11.0),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                )],
            ))
            .id();
        commands.entity(panel).add_child(more);
    }
    panel
}

/// One row of the stack panel: the object, what it is, and what it points at.
///
/// `full` is the top of the stack — the sentence the panel is about. Every
/// other row is the queue behind it and is drawn at two thirds, with no
/// subtitle and no arrow: a queued entry answers "what else is coming", and
/// the seat and the kind are questions a player asks about the thing that is
/// *next*.
#[allow(clippy::too_many_arguments)] // the same slot arguments, one level down
#[allow(clippy::too_many_lines)] // two shapes of one row, built flat
fn spawn_stack_entry(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    full: bool,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let key = StackKey::Entry(item.id, full);
    // The next thing to resolve is lit and railed; the one behind it carries
    // a hint of the same fill so "this resolves second" is visible, and
    // everything under *that* is flat. Depth is the only ordering a player
    // has to trust here, and past the second row it is carried by position.
    let fill = if full {
        palette::PANEL_LIT
    } else if item.depth == 1 {
        palette::PANEL_LIT.with_alpha(0.45)
    } else {
        Color::NONE
    };
    let rail = if full { palette::ACCENT } else { Color::NONE };
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                padding: UiRect::all(px(if full { 6.0 } else { 4.0 })),
                border: UiRect::left(px(3)),
                align_items: AlignItems::FlexStart,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(rail),
            Pickable::IGNORE,
            Arriving::fill(key, fill.alpha()),
            ArrivingRow {
                lift: ARRIVE_LIFT,
                from: ARRIVE_SCALE,
                rail,
            },
        ))
        .id();

    let (width, height) = if full {
        (STACK_CARD_W, STACK_CARD_H)
    } else {
        (STACK_QUEUED_W, STACK_QUEUED_H)
    };
    let art = spawn_stack_card(
        commands,
        lang,
        key,
        Some(item.id),
        item.art,
        stack_face(item, view, faces, textures),
        width,
        height,
        statics,
        textures,
        assets,
        fonts,
        cards.as_deref_mut(),
    );
    commands.entity(row).add_child(art);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(body);

    // The body is the panel less its padding, the row's own padding, the rail
    // and the card; a name is cut to fit it on one line rather than wrapped,
    // because a two-line name in a queued row breaks the height the whole
    // panel is budgeted against.
    let ink = if full { palette::ACCENT } else { palette::INK };
    let size = if full { 15.0 } else { 13.0 };
    let room = STACK_PANEL_W - 20.0 - if full { 12.0 } else { 8.0 } - 3.0 - width - 8.0;
    let name = commands
        .spawn((
            Text::new(fit(&item.name, room, size)),
            tf(fonts, size),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            TextColor(ink),
            Arriving::ink(key, ink.alpha()),
        ))
        .id();
    commands.entity(body).add_child(name);

    // What kind of thing this is, and whose — on the top row only. An ability
    // names its source even when the source has left: the picture above may
    // be missing, the sentence must not be. A *spell* names nothing, because
    // the picture already said it.
    if full {
        let kind = match item.kind {
            baylee_client_core::board::StackKind::Spell => None,
            baylee_client_core::board::StackKind::Ability { source } => {
                Some(view.object(source).map_or_else(
                    || Phrase::StackAbilityBare.text(lang).to_string(),
                    |o| Phrase::StackAbility.fill(lang, &[&o.name]),
                ))
            }
        };
        let subtitle = commands
            .spawn((
                Text::default(),
                tf(fonts, 11.0),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        if let Some(kind) = kind {
            let span = commands
                .spawn((
                    TextSpan::new(format!("{kind} — ")),
                    tf(fonts, 11.0),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                ))
                .id();
            commands.entity(subtitle).add_child(span);
        }
        // The seat in the slant, and only the seat: it is the one word of the
        // subtitle that names a *person*, and italic Inter at eleven pixels
        // over this ground is legible for a word and tiring for a sentence.
        let seat = commands
            .spawn((
                TextSpan::new(statics.seat_name(item.controller).to_string()),
                tf_italic(fonts, 11.0),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
            ))
            .id();
        commands.entity(subtitle).add_child(seat);
        commands.entity(body).add_child(subtitle);
    }

    if !item.targets.is_empty() {
        let targets = spawn_stack_targets(
            commands, lang, item, key, full, statics, textures, assets, fonts, cards,
        );
        commands.entity(body).add_child(targets);
    }

    row
}

/// The arrow and everything a stack entry points at, as one wrapping row.
#[allow(clippy::too_many_arguments)] // the same slot arguments again
fn spawn_stack_targets(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    key: StackKey,
    full: bool,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(4),
                margin: UiRect::top(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // The arrow belongs to the sentence, not to the queue: a queued row is
    // already the narrow one, and a glyph that is a third of its body width
    // buys nothing a thumbnail sitting under a name does not already say.
    if full {
        let arrow = commands
            .spawn((
                Text::new("→"),
                tf(fonts, 14.0),
                TextColor(palette::ACCENT),
                Arriving::ink(key, palette::ACCENT.alpha()),
            ))
            .id();
        commands.entity(row).add_child(arrow);
    }

    for target in &item.targets {
        let chip = spawn_stack_target(
            commands,
            lang,
            target,
            key,
            full,
            statics,
            textures,
            assets,
            fonts,
            cards.as_deref_mut(),
        );
        commands.entity(row).add_child(chip);
    }
    row
}

/// The face a stack entry should show instead of its art, if any.
///
/// A spell on the stack has its own object and its own projected text; an
/// ability has neither, so it keeps its picture and its name.
fn stack_face(
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
    textures: &CardTextures,
) -> Option<CardFace> {
    let object = view.object(item.id)?;
    faces.object(object, textures, item.art)
}

/// A card in the stack panel: its art, its face, or a plain plate with the
/// card's back colour when the seat may not know what it is.
#[allow(clippy::too_many_arguments)] // the slot, the card, and the stores
fn spawn_stack_card(
    commands: &mut Commands,
    lang: Lang,
    key: StackKey,
    object: Option<ObjectId>,
    art: Option<ImageKey>,
    face: Option<CardFace>,
    width: f32,
    height: f32,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                border_radius: card_radius(width),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            soft_shadow(),
            Arriving::fill(key, palette::PANEL_LIT.alpha()),
        ))
        .id();
    // The one pickable thing in this panel. A stack card is drawn an inch
    // across — enough to recognise a spell, nowhere near enough to read one —
    // and the stack is where a player most needs to read. `HandCardVisual` is
    // what `pointer_hover` looks for, and it already speaks for every card
    // the HUD draws rather than the felt; a card whose object this seat may
    // not know (something cast face down) reports nothing and simply does not
    // preview.
    match object {
        Some(object) => {
            commands.entity(slot).insert(HandCardVisual { object });
        }
        None => {
            commands.entity(slot).insert(Pickable::IGNORE);
        }
    }
    if let Some(key) = art {
        let image = textures.get(key, statics, assets);
        let visual = spawn_card_art(
            commands,
            lang,
            image,
            face.as_ref(),
            width,
            height,
            crate::face::Detail::Compact,
            fonts,
            // Nothing on the stack is on a battlefield, so no glow: the
            // border says what the rules have made a *permanent*, and a
            // spell wearing one would be claiming something untrue.
            CardLook::art(key, finish_of(statics, Some(key)), 0),
            cards,
        );
        commands.entity(slot).add_child(visual);
    }
    // The picture's own fade, laid over it rather than mixed into it. The art
    // is a `MaterialNode` on a material shared with every card that looks the
    // same, so an alpha written there would fade the hand as well; a veil the
    // colour of the empty slot fades exactly this one, and costs one node.
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT.with_alpha(0.0)),
            Pickable::IGNORE,
            Arriving::veil(key),
        ))
        .id();
    commands.entity(slot).add_child(veil);
    slot
}

/// One target of a stack entry: its picture when it has one, a chip when it
/// does not — a player, a token this seat cannot name, a face-down permanent.
#[allow(clippy::too_many_arguments)] // as above
fn spawn_stack_target(
    commands: &mut Commands,
    lang: Lang,
    target: &baylee_client_core::board::StackTarget,
    key: StackKey,
    full: bool,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let (width, height) = if full {
        (STACK_TARGET_W, STACK_TARGET_H)
    } else {
        (STACK_QUEUED_TARGET_W, STACK_QUEUED_TARGET_H)
    };
    if target.art.is_some() {
        return spawn_stack_card(
            commands,
            lang,
            key,
            target.object(),
            target.art,
            None,
            width,
            height,
            statics,
            textures,
            assets,
            fonts,
            cards,
        );
    }
    // A player is a name and a heart, not a rectangle pretending to be a
    // card. Anything else with no picture (a token, something face down) is
    // its name in the same chip, so the row never has a hole in it.
    let (glyph, label) = match target.player() {
        Some(player) => (Some(glyph::HEART), statics.seat_name(player).to_string()),
        None => (None, target.name.clone().unwrap_or_else(|| "?".into())),
    };
    let size = if full { 12.0 } else { 11.0 };
    let chip = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(6.0), px(3.0)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
            Arriving::fill(key, palette::PANEL_LIT.alpha()),
        ))
        .id();
    if let Some(glyph) = glyph {
        let icon = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(fonts, size - 2.0),
                TextColor(palette::DANGER),
                Arriving::ink(key, palette::DANGER.alpha()),
            ))
            .id();
        commands.entity(chip).add_child(icon);
    }
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, size),
            TextColor(palette::INK),
            Arriving::ink(key, palette::INK.alpha()),
        ))
        .id();
    commands.entity(chip).add_child(text);
    chip
}

/// A card name cut to one line `room` pixels wide, set at `size`.
///
/// Inter's lower case averages a little over half its point size and a card
/// name is mostly lower case, so `0.52` is the ratio the budget is taken at —
/// deliberately generous, because the cost of guessing narrow is one word
/// clipped by the body's own `overflow` and the cost of guessing wide is a
/// name cut short that would have fitted.
///
/// The ellipsis replaces characters rather than joining them, so the result
/// never grows past the budget, and the cut is on `char` boundaries because a
/// card name is not ASCII (Æther Vial, Márton Stromgald).
fn fit(name: &str, room: f32, size: f32) -> String {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let budget = (room / (size * 0.52)).max(4.0) as usize;
    if name.chars().count() <= budget {
        return name.to_string();
    }
    let mut cut: String = name.chars().take(budget - 1).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name that fits is left exactly as printed — the common case, and the
    /// one where a stray ellipsis would be a lie about the card.
    #[test]
    fn a_short_name_is_not_cut() {
        assert_eq!(fit("Shock", 187.0, 15.0), "Shock");
    }

    /// A long one is cut *and* stays inside the budget it was cut to. The
    /// first attempt appended the ellipsis to a full-budget slice and came
    /// out one character wider than the room it was given.
    #[test]
    fn a_long_name_is_cut_to_fit() {
        let room = 187.0;
        let size = 15.0;
        let cut = fit("Asmoranomardicadaistinaculdacar", room, size);
        assert!(cut.ends_with('…'), "cut without saying so: {cut}");
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let budget = (room / (size * 0.52)) as usize;
        assert!(
            cut.chars().count() <= budget,
            "{cut} is {} chars, over the {budget} it had",
            cut.chars().count()
        );
    }

    /// The cut lands on character boundaries. A byte-wise slice of a name
    /// with an accent in it panics, and the pool has several.
    #[test]
    fn a_name_that_is_not_ascii_survives_the_cut() {
        let cut = fit("Æther Vial of Márton Stromgald’s Æther", 60.0, 13.0);
        assert!(cut.chars().count() < 20, "{cut} was not cut at all");
    }

    /// A queued row is the same object as the full row it will be promoted
    /// to, and a *different* key — which is the whole reason a resolution
    /// eases instead of cutting.
    #[test]
    fn promotion_is_a_new_key() {
        let id = ObjectId::new(7, 0);
        assert_ne!(StackKey::Entry(id, false), StackKey::Entry(id, true));
        assert_eq!(StackKey::Entry(id, true), StackKey::Entry(id, true));
    }

    /// Progress is remembered per row and forgotten for a row that is not on
    /// screen, so a rebuilt panel re-attaches to the arrival in progress and
    /// an object that resolved leaves nothing behind.
    #[test]
    fn a_row_that_is_not_tracked_is_simply_drawn() {
        let mut motion = StackMotion::default();
        let id = ObjectId::new(3, 0);
        assert!((motion.progress(StackKey::Entry(id, true)) - 1.0).abs() < f32::EPSILON);
        motion.rows.push((StackKey::Entry(id, true), 0.4));
        assert!((motion.progress(StackKey::Entry(id, true)) - 0.4).abs() < f32::EPSILON);
        assert!((motion.progress(StackKey::Panel) - 1.0).abs() < f32::EPSILON);
    }

    /// A veil is the inverse of ink: opaque while the row arrives, gone once
    /// it has. Without that the card picture — which is on a shared material
    /// and cannot be faded itself — would pop in at full strength.
    #[test]
    fn a_veil_clears_as_the_ink_comes_up() {
        let key = StackKey::Panel;
        let ink = Arriving::ink(key, 0.88);
        let veil = Arriving::veil(key);
        assert!(ink.alpha(0.0).abs() < f32::EPSILON);
        assert!((veil.alpha(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((ink.alpha(1.0) - 0.88).abs() < f32::EPSILON);
        assert!(veil.alpha(1.0).abs() < f32::EPSILON);
    }

    // ---- the system, actually run ---------------------------------------
    //
    // The arithmetic above is the easy half. "Declared but never wired" is a
    // bug this client has shipped before, so the rest of these run the system
    // in an `App` and assert on what a frame left behind.

    use crate::prefs::Prefs;

    /// An app with the system in it and nothing else.
    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Prefs>()
            .init_resource::<StackMotion>()
            .add_systems(Update, ease_the_stack_in);
        app
    }

    /// One row: a fill that fades and a transform that lifts, plus a line of
    /// text under it, which is the part no opacity inheritance would reach.
    fn a_row(app: &mut App, key: StackKey) -> (Entity, Entity) {
        let row = app
            .world_mut()
            .spawn((
                Node::default(),
                BackgroundColor(palette::PANEL_LIT),
                Arriving::fill(key, palette::PANEL_LIT.alpha()),
                ArrivingRow {
                    lift: ARRIVE_LIFT,
                    from: ARRIVE_SCALE,
                    rail: palette::ACCENT,
                },
            ))
            .id();
        let ink = app
            .world_mut()
            .spawn((
                Node::default(),
                TextColor(palette::INK),
                Arriving::ink(key, palette::INK.alpha()),
            ))
            .id();
        (row, ink)
    }

    /// A sixtieth of a second, the frame this client is tuned against.
    fn a_frame(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.update();
    }

    fn alpha_of(app: &App, row: Entity) -> f32 {
        app.world()
            .entity(row)
            .get::<BackgroundColor>()
            .unwrap()
            .0
            .alpha()
    }

    fn lift_of(app: &App, row: Entity) -> f32 {
        match app
            .world()
            .entity(row)
            .get::<UiTransform>()
            .unwrap()
            .translation
            .y
        {
            Val::Px(y) => y,
            other => panic!("the lift is not in pixels: {other:?}"),
        }
    }

    /// A row comes up from nothing, over several frames, and then holds
    /// exactly where it belongs. Both ends matter: a fade that never finished
    /// would leave the panel permanently dim.
    #[test]
    fn a_row_arrives_and_then_holds_still() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(1, 0), true);
        let (row, ink) = a_row(&mut app, key);

        a_frame(&mut app);
        let part = alpha_of(&app, row);
        assert!(
            part > 0.0 && part < palette::PANEL_LIT.alpha(),
            "one frame in, the row is part way: {part}"
        );
        assert!(lift_of(&app, row) < -0.5, "and still above its place");
        let text = app
            .world()
            .entity(ink)
            .get::<TextColor>()
            .unwrap()
            .0
            .alpha();
        assert!(
            text > 0.0 && text < 1.0,
            "the text fades with it, having no inheritance to ride: {text}"
        );

        for _ in 0..40 {
            a_frame(&mut app);
        }
        assert!(
            (alpha_of(&app, row) - palette::PANEL_LIT.alpha()).abs() < 1e-4,
            "the row lands at the alpha it was drawn in"
        );
        assert!(lift_of(&app, row).abs() < 1e-4, "and at its resting place");
        let scale = app.world().entity(row).get::<UiTransform>().unwrap().scale;
        assert!((scale.x - 1.0).abs() < 1e-4, "at full size: {scale:?}");
    }

    /// The whole reason the progress is a resource: the HUD is rebuilt on
    /// hover, and a row that restarted its fade every time the pointer moved
    /// would flicker instead of arriving.
    #[test]
    fn a_rebuilt_row_carries_on_where_it_was() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(2, 0), false);
        let (row, _) = a_row(&mut app, key);
        for _ in 0..4 {
            a_frame(&mut app);
        }
        let midway = alpha_of(&app, row);
        assert!(midway > 0.1, "far enough in to tell a restart from a fade");

        // What a `HudRevision` change does: the whole subtree goes and is
        // built again from scratch, at rest, with the same key.
        app.world_mut().entity_mut(row).despawn();
        let (again, _) = a_row(&mut app, key);
        a_frame(&mut app);
        assert!(
            alpha_of(&app, again) > midway,
            "the rebuild continued the arrival rather than restarting it"
        );
    }

    /// A row that steps down holds still.
    ///
    /// This is the headline case — a spell lands on a stack that already had
    /// one — and the key carries the row's shape, so the object that was on
    /// top is under a *new* key the moment it is drawn queued. Seeded like an
    /// arrival it would fade in beside the newcomer and the player would see
    /// two spells land where one did.
    #[test]
    fn a_row_that_steps_down_does_not_announce_itself() {
        let mut app = harness();
        let old = ObjectId::new(8, 0);
        let (top, top_ink) = a_row(&mut app, StackKey::Entry(old, true));
        for _ in 0..40 {
            a_frame(&mut app);
        }

        // A spell lands: the panel is rebuilt, the old top in the queued
        // shape and the newcomer full above it.
        app.world_mut().entity_mut(top).despawn();
        app.world_mut().entity_mut(top_ink).despawn();
        let (stepped, _) = a_row(&mut app, StackKey::Entry(old, false));
        let (landed, _) = a_row(&mut app, StackKey::Entry(ObjectId::new(9, 0), true));
        a_frame(&mut app);

        assert!(
            (alpha_of(&app, stepped) - palette::PANEL_LIT.alpha()).abs() < 1e-4,
            "the demoted row stood where it was"
        );
        assert!(lift_of(&app, stepped).abs() < 1e-4, "and did not drop in");
        let arriving = alpha_of(&app, landed);
        assert!(
            arriving > 0.0 && arriving < palette::PANEL_LIT.alpha(),
            "while the spell that did land is still arriving: {arriving}"
        );
    }

    /// The other direction is not carried, and that is the point: a queued
    /// row becoming full is what a resolution looks like from the panel.
    #[test]
    fn a_row_that_is_promoted_still_arrives() {
        let mut app = harness();
        let id = ObjectId::new(10, 0);
        let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
        for _ in 0..40 {
            a_frame(&mut app);
        }
        app.world_mut().entity_mut(queued).despawn();
        app.world_mut().entity_mut(queued_ink).despawn();
        let (full, _) = a_row(&mut app, StackKey::Entry(id, true));
        a_frame(&mut app);
        let part = alpha_of(&app, full);
        assert!(
            part > 0.0 && part < palette::PANEL_LIT.alpha(),
            "a resolution is meant to be seen: {part}"
        );
    }

    /// A player who has asked for stillness gets the panel, not the arrival.
    #[test]
    fn holding_still_puts_the_row_straight_where_it_belongs() {
        let mut app = harness();
        app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = true;
        let key = StackKey::Entry(ObjectId::new(3, 0), true);
        let (row, _) = a_row(&mut app, key);
        a_frame(&mut app);
        assert!(
            (alpha_of(&app, row) - palette::PANEL_LIT.alpha()).abs() < 1e-4,
            "there on the first frame"
        );
        assert!(lift_of(&app, row).abs() < 1e-4);
    }

    /// A line of text keeps the background it does not have.
    ///
    /// [`Node`] requires a [`BackgroundColor`], so every label in this panel
    /// carries a transparent one; a fade that wrote whichever colour it found
    /// turned each of those into an opaque black plate, and the first live
    /// shot of the panel was eight of them where the words should be. The
    /// [`Paints`] tag is what stops it, and this is the test that would have
    /// caught it — the arithmetic tests all passed while the panel was
    /// unreadable.
    #[test]
    fn fading_a_label_does_not_give_it_a_plate() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(5, 0), true);
        let (_, ink) = a_row(&mut app, key);
        for _ in 0..3 {
            a_frame(&mut app);
        }
        let plate = app
            .world()
            .entity(ink)
            .get::<BackgroundColor>()
            .expect("a Node always has one")
            .0;
        assert!(
            plate.alpha().abs() < f32::EPSILON,
            "the label grew a background: {plate:?}"
        );
        assert!(
            app.world()
                .entity(ink)
                .get::<TextColor>()
                .unwrap()
                .0
                .alpha()
                > 0.0,
            "while its ink did come up"
        );
    }

    /// The accent rail arrives with the row rather than standing there alone
    /// while the row fades in behind it — and a row with no rail never grows
    /// one, which is the same `Color::NONE` trap one component along.
    #[test]
    fn the_rail_arrives_with_its_row() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(6, 0), true);
        let (row, _) = a_row(&mut app, key);
        a_frame(&mut app);
        let part = app.world().entity(row).get::<BorderColor>().unwrap().left;
        assert!(
            part.alpha() > 0.0 && part.alpha() < 1.0,
            "the rail comes up with the row: {part:?}"
        );
        for _ in 0..40 {
            a_frame(&mut app);
        }
        let rested = app.world().entity(row).get::<BorderColor>().unwrap().left;
        assert!((rested.alpha() - 1.0).abs() < 1e-4, "and lands lit");
    }

    /// A resolved spell leaves nothing behind. Without the prune, the *next*
    /// object at that id and shape would find its arrival already finished
    /// and appear with no animation at all.
    #[test]
    fn a_row_that_leaves_the_panel_is_forgotten() {
        let mut app = harness();
        let key = StackKey::Entry(ObjectId::new(4, 0), true);
        let (row, ink) = a_row(&mut app, key);
        a_frame(&mut app);
        assert_eq!(app.world().resource::<StackMotion>().rows.len(), 1);

        app.world_mut().entity_mut(row).despawn();
        app.world_mut().entity_mut(ink).despawn();
        a_frame(&mut app);
        assert!(
            app.world().resource::<StackMotion>().rows.is_empty(),
            "the panel emptied and took its progress with it"
        );
    }
}
