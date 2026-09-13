//! The whole sentence a stack entry stands for, written on parchment under
//! the hover preview.
//!
//! # Why it is not the row, and not the card either
//!
//! The stack panel abbreviates. A **queued** row draws no sentence at all and
//! a **full** row draws one cut to [`STACK_SENTENCE_LINES`] lines, which is
//! the arithmetic that lets a stack of ten stay one panel — and it is exactly
//! what the owner reported: *"Auf dem Stack werden die Effekte oft
//! abgekürzt."* The place that answers it is the hover preview, because
//! `docs/redesign-proposal.md` §8.2 already decided that the preview "is a
//! description of the card and is the only place rules text is read at size".
//! A second such place inside the panel would be two answers to one question,
//! and it has a concrete failure besides: the panel is rebuilt on every hover
//! change, so a row that grew under the pointer could push the pointer out of
//! itself, which changes the hover, which rebuilds the row collapsed — a
//! two-frame oscillation the `grace` counter in `pointer_hover` would not
//! suppress, because the pointer really has moved relative to the tree.
//!
//! But the card's own text face is not the answer either, and that is the
//! spine of this module. **A card prints several abilities; the stack holds
//! one.** [`super::stack::stack_sentence`] already resolves which — that is
//! the sentence the row cuts — so what goes under the picture is *that*
//! sentence whole, in the player's own printing and language, and not the
//! whole card. For a **spell** it is the cast face's rules text, because on
//! the stack the object *is* its text about to resolve: the stack is the one
//! zone where "what does this card look like" is never the question.
//!
//! The one exception is the modifier: when the bubble is already showing the
//! text face by choice ([`super::card::FaceCtx::always`]), the face *is* the
//! slip and drawing a second one would be the same words twice. It is asked
//! about the *choice* and not about `preview_face` returning something,
//! because a face is also what fills in while art is still in flight — which
//! is the ordinary case offline, and the case where a card scan with English
//! rules text on it looks as though this feature already exists.
//!
//! # The wash
//!
//! Three stages, and the middle one is the only thing in this client that is
//! not an exponential:
//!
//! 1. **The ground.** The sheet comes out of the card's foot — alpha up and
//!    [`SLIP_LIFT`] pixels down into place — at [`crate::ambience::FEEL_RATE`],
//!    because it answers a pointer and anything slower there reads as lag.
//! 2. **The ink.** Text alpha up at the same rate, gated on the ground being
//!    half there, under a veil whose edge travels down the page. That travel
//!    is **linear over a duration** ([`across`]) and is a deliberate
//!    departure: an exponential wipe rushes the first line and crawls through
//!    the last, and reading order is the whole reason the wipe exists.
//! 3. **The tone.** The ink lands a quarter of the way back towards the sheet
//!    and dries to its own colour at [`SETTLE_RATE`]. Wet ink is lighter and
//!    browner; this is the same land-bright-cool-slow grammar the promoted
//!    stack row's rail already uses, so it is not new vocabulary.
//!
//! Legible at about 200 ms and finished at about 450.
//!
//! # Where the progress lives
//!
//! Not on the nodes: the overlay is a retained tree rebuilt whenever
//! [`HudRevision`] changes, and *hover* is part of that gate — so the slip is
//! despawned and respawned by the very pointer movement that opened it. It
//! lives in [`SlipWash`] instead, for the same reason [`super::stack::StackMotion`]
//! exists, and the nodes are spawned **at rest** so a rebuild after the wash
//! has finished draws nothing dim.
//!
//! The brief this was built from proposed hanging it off `crate::sheen::Sheen`
//! instead, which already keys the preview's own sweep under
//! `(id, Surface::Preview)`. That does not reach: `Sheen` drops a sweep as
//! soon as it is done, and a burst floors a sweep at 340 ms — under the 450
//! this takes — so the last third of the wash would have no clock.
//!
//! [`STACK_SENTENCE_LINES`]: super::stack
//! [`HudRevision`]: super::HudRevision
//! [`SETTLE_RATE`]: super::stack

use super::stack::Piece;
#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::card_face::TextBlock;
use bevy::color::Mix;

/// The size the sentence is set at.
///
/// One point over the stack row's, and the point is the ramp: the row is a
/// queue position and this is the thing being read.
const SLIP_PT: f32 = 13.0;

/// The line box that text sits in, as a multiple of its size.
const SLIP_LEADING: f32 = 1.25;

/// One line of it, in pixels.
const SLIP_LINE: f32 = SLIP_PT * super::SERIF_SCALE * SLIP_LEADING;

/// The heading — what kind of thing this is, and whose.
const SLIP_HEAD_PT: f32 = 11.0;

/// Its line box.
const SLIP_HEAD_LINE: f32 = SLIP_HEAD_PT * super::UI_SCALE * SLIP_LEADING;

/// The gap between the heading and the sentence.
const SLIP_ROW_GAP: f32 = 5.0;

/// The gap between two printed paragraphs.
///
/// Wider than [`SLIP_ROW_GAP`]: the heading and the sentence are one block
/// with a label on it, while two paragraphs are two things a card says, and
/// on a planeswalker they are two separate abilities a player is choosing
/// between.
const SLIP_PARA_GAP: f32 = 7.0;

/// The margin of sheet around the writing.
const SLIP_PAD: f32 = 10.0;

/// The gap between the card's foot and the sheet under it.
const SLIP_GAP: f32 = 6.0;

/// How far above its place the sheet starts, in logical pixels.
///
/// Down rather than up: it comes *out of* the card above it, which is what
/// makes the two read as one object rather than as a panel that appeared.
const SLIP_LIFT: f32 = 6.0;

/// A mana mark in the sentence, as a fraction of the prose's size — the same
/// ratio and for the same reason as [`super::stack::STACK_MARK`].
const SLIP_MARK: f32 = 0.72;

/// How far past the line it is revealing the wipe's soft edge reaches.
///
/// In pixels rather than a percentage, so the fade is the same thickness on a
/// one-line ability and a nine-line spell: a percentage would make a long
/// card's edge nine times softer than a short one's, which is a different
/// animation for the same event.
const WIPE_SOFT: f32 = 20.0;

/// How long the wipe spends on each line of text.
const WIPE_PER_LINE: f32 = 0.040;

/// The shortest and longest the whole wipe may take.
///
/// The floor keeps a one-line trigger from being a flicker; the ceiling keeps
/// a nine-line spell from being a thing the player waits for. Between them is
/// every card in the pool.
const WIPE_MIN: f32 = 0.120;
const WIPE_MAX: f32 = 0.320;

/// How far the ground has to be there before the ink starts.
///
/// Half, which at [`crate::ambience::FEEL_RATE`] is about 40 ms — long enough
/// that the sheet is visibly the thing the writing lands on, short enough
/// that it is one movement rather than two.
const INK_STARTS: f32 = 0.5;

/// How fast the ink dries, per second. The stack panel's own settle rate: a
/// light that lands bright and cools is one idea, written once.
const DRY_RATE: f32 = 8.0;

/// How far back towards the sheet wet ink is.
///
/// A mix rather than a named colour, because the sentence has two inks — the
/// rules text and the quieter aside — and both have to dry the same way. For
/// [`palette::SLIP_INK`] a quarter lands within a hundredth and a half of
/// [`palette::PARCHMENT_SOFT`] on every channel, which is where the number
/// comes from; the pair are not collinear with the sheet, so it cannot be
/// exact and a named colour would only fit one of the two.
const WET: f32 = 0.25;

/// The tallest a slip may be, as a fraction of the window.
///
/// A clip for a printing this client has never seen, not a budget for
/// anything in the pool: the longest ability is 113 characters and the
/// longest spell text about nine lines, both well under this.
const SLIP_MAX_H: f32 = 0.40;

/// What the slip says, and about what.
pub(super) struct Slip {
    /// The object whose sentence this is. The wash is keyed on it, so walking
    /// down the panel re-inks the slip without rebuilding the sheet.
    pub says: ObjectId,
    /// What kind of thing it is and where it came from — `None` for a spell,
    /// whose picture is directly above the slip. An **ability** names its
    /// source even when the source has left (CR 113.7a): the picture may be
    /// missing, the sentence must not be.
    pub kind: Option<String>,
    /// Whose it is.
    pub seat: String,
    /// The sentence, or the whole printed body — **by paragraph**.
    ///
    /// Two levels and not one, because a card prints paragraphs and
    /// [`baylee_client_core::card_face::split_blocks`] does not say where
    /// they ended: it splits on the newline and on the reminder's brackets
    /// alike, and hands back one flat list. Drawn flat, a planeswalker's
    /// three loyalty abilities run together as
    /// `…oben auf deine Bibliothek.−1: Schicke…`, which was the first thing
    /// the live shot showed.
    pub blocks: Vec<Vec<TextBlock>>,
}

/// What the slip under the preview should say, if anything.
///
/// `None` for everything that is not an object on the stack — a card in hand,
/// a permanent, a pile's top card, a row in the zone browser. A stack entry's
/// *target* qualifies only when it is itself on the stack, which is the case
/// a counterspell makes and is right: the same object is about to resolve.
pub(super) fn says(
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    lang: Lang,
    faces: &FaceCtx<'_>,
    hovered: Option<ObjectId>,
) -> Option<Slip> {
    // The face is the slip when the player has asked for it. Not
    // `preview_face(..).is_some()`: that is also how a card with no art yet
    // is drawn, which is every card offline.
    if faces.always() {
        return None;
    }
    let id = hovered?;
    let item = board.stack.iter().find(|item| item.id == id)?;
    let seat = statics.seat_name(item.controller).to_string();
    match item.kind {
        baylee_client_core::board::StackKind::Ability { source, .. } => {
            let blocks = super::stack::stack_sentence(item, view, faces)?;
            let kind = view.object(source).map_or_else(
                || Phrase::StackAbilityBare.text(lang).to_string(),
                |o| Phrase::StackAbility.fill(lang, &[&crate::face::name_of(o, view, faces.texts)]),
            );
            Some(Slip {
                says: id,
                kind: Some(kind),
                // An ability on the stack is one sentence, which is one
                // paragraph by construction.
                seat,
                blocks: vec![blocks],
            })
        }
        baylee_client_core::board::StackKind::Spell => {
            let card = view.object(id)?.card?;
            let text = faces.texts.get(card.print, card.face)?;
            // Split by paragraph *here* and hand each one to `split_blocks`
            // on its own, which is what keeps the boundary the flat list
            // loses. `split_blocks` over one paragraph is exactly the
            // reminder split, which is what a paragraph wants.
            let blocks: Vec<Vec<TextBlock>> = text
                .oracle_text
                .split('\n')
                .map(baylee_client_core::card_face::split_blocks)
                .filter(|para| !para.is_empty())
                .collect();
            // A vanilla creature spell prints no rules text at all, and an
            // empty sheet under the picture would be a panel saying nothing.
            if blocks.is_empty() {
                return None;
            }
            Some(Slip {
                says: id,
                kind: None,
                seat,
                blocks,
            })
        }
    }
}

/// Every run of every paragraph, as they are drawn — no budget, because the
/// slip wraps rather than cutting.
pub(super) fn runs(slip: &Slip) -> Vec<Vec<Piece>> {
    slip.blocks
        .iter()
        .map(|para| super::stack::spans_of(para, None))
        .collect()
}

/// How many lines `runs` wraps to in a sheet `width` pixels wide.
///
/// Counted per paragraph and summed, because each one starts a line of its
/// own: a card of four short paragraphs is four lines tall, not one.
///
/// The estimator is the same character guess the stack row's names are cut
/// with — a claim about the shipped faces' advance widths — and is used for a
/// *duration* and a placement rather than for a cut, so being a character or
/// two out costs a few milliseconds of wipe and nothing else.
fn lines(runs: &[Vec<Piece>], width: f32) -> usize {
    let per = super::stack::budget(width - 2.0 * SLIP_PAD, SLIP_PT).max(1);
    runs.iter()
        .map(|para| {
            para.iter()
                .map(|piece| piece.text.chars().count())
                .sum::<usize>()
                .div_ceil(per)
                .max(1)
        })
        .sum::<usize>()
        .max(1)
}

/// How long the wipe takes for a sentence of `lines` lines.
#[allow(clippy::cast_precision_loss)] // a nine-line card is the long tail
fn across(lines: usize) -> f32 {
    (lines as f32 * WIPE_PER_LINE).clamp(WIPE_MIN, WIPE_MAX)
}

/// How tall the sheet will be, for the placement arithmetic that has to know
/// before anything is laid out.
///
/// An estimate, and it is allowed to be: [`super::hand::preview_place`] uses
/// it to decide which side of the pointer the bubble opens on, and the node
/// itself is laid out by `bevy_ui` from the text that is actually in it.
pub(super) fn height(runs: &[Vec<Piece>], width: f32, heading: bool) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let body = lines(runs, width) as f32 * SLIP_LINE
        + (runs.len().saturating_sub(1)) as f32 * SLIP_PARA_GAP;
    let head = if heading {
        SLIP_HEAD_LINE + SLIP_ROW_GAP
    } else {
        0.0
    };
    SLIP_GAP + 2.0 * SLIP_PAD + head + body
}

/// Which part of the slip a node is, and therefore what the wash writes to
/// it.
///
/// Load-bearing in the same way [`super::stack::Arriving`]'s `Paints` is:
/// every UI node carries a [`BackgroundColor`] whether it draws one or not,
/// so a system that simply wrote whichever colour it found would give every
/// line of the sentence an opaque plate.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// The sheet: its own fill, and the pixels it slides down.
    Ground,
    /// The parchment grain over that fill, which is an [`ImageNode`] and
    /// fades through its tint.
    Grain,
    /// The heading, which arrives with the ground rather than with the ink —
    /// it says what the slip is about and is not part of what is being read.
    Heading,
    /// A run of the sentence: alpha from the ink, colour from the drying.
    Ink,
    /// The wipe, which is a `top` and nothing else.
    Veil,
}

/// A node of the slip, and what the wash owes it.
#[derive(Component)]
pub struct Washing {
    part: Part,
    /// The colour the node rests at once the ink is dry.
    base: Color,
    /// What the slip is saying. Carried on every node rather than looked up,
    /// for the reason [`super::stack::Arriving`] carries its key: a node that
    /// knows its own slip needs no tree walk to be found.
    says: ObjectId,
    /// How long this sentence's wipe takes.
    across: f32,
}

/// How far the slip has been written, `0.0`…`1.0` in four numbers.
///
/// One slip at a time, so this is a struct and not a map: the preview shows
/// one card, and the pointer is in one place.
#[derive(Resource, Default)]
pub struct SlipWash {
    /// What the slip on screen is saying, or `None` when there is no slip.
    saying: Option<ObjectId>,
    /// The sheet: alpha and the slide.
    ground: f32,
    /// The ink's alpha.
    ink: f32,
    /// How far the wipe has crossed the page.
    wipe: f32,
    /// How far the ink has dried.
    dried: f32,
}

impl SlipWash {
    /// Everything at rest, which is what a node is spawned at.
    fn done() -> Self {
        Self {
            saying: None,
            ground: 1.0,
            ink: 1.0,
            wipe: 1.0,
            dried: 1.0,
        }
    }

    /// Whether anything is still moving.
    fn moving(&self) -> bool {
        self.ground < 1.0 || self.ink < 1.0 || self.wipe < 1.0 || self.dried < 1.0
    }
}

/// One exponential step towards rest, with the last thousandth snapped shut.
///
/// The snap is not cosmetic. An exponential only ever *approaches* one, so
/// without it [`SlipWash::moving`] is true for the rest of the game and every
/// node of the slip is written to on every frame forever — and the sheet
/// never actually lands, which is a sixth of a pixel of permanent lift.
fn towards(at: f32, step: f32) -> f32 {
    let at = (at + (1.0 - at) * step).min(1.0);
    if at > 0.999 { 1.0 } else { at }
}

/// Writes the slip on, once a frame.
///
/// Runs **after** `sync_overlay` for the reason `ease_the_stack_in` does: a
/// slip spawned this frame is spawned at rest, and without the ordering it
/// would be drawn once, fully written, before the wash ever saw it — a flash
/// at exactly the place the player is looking.
pub fn wash_the_slip_in(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut wash: ResMut<SlipWash>,
    // Read-only and first, because what the wash does this frame depends on
    // *which* slip is on screen, and the two queries below write.
    slips: Query<&Washing>,
    // Two queries and not four, and the split is by what is written rather
    // than by which part is being written: every node here carries a `Node`,
    // and `Node` requires a `BackgroundColor` and a `UiTransform`, so the
    // sheet and the wipe are one query. The other two are the optional
    // halves — a line of text has no [`ImageNode`] and the grain has no
    // [`TextColor`] — and neither component appears in the first, so the two
    // queries cannot conflict over anything.
    mut shapes: Query<(&Washing, &mut Node, &mut BackgroundColor, &mut UiTransform)>,
    mut paints: Query<(&Washing, Option<&mut TextColor>, Option<&mut ImageNode>)>,
) {
    // Every node of one slip carries the same two facts, so any of them
    // answers. An empty query is the pointer having left the stack, and the
    // wash goes with the slip — a progress kept after the bubble closed would
    // greet the next hover with no animation at all.
    let Some((says, across)) = slips.iter().next().map(|w| (w.says, w.across)) else {
        *wash = SlipWash::default();
        return;
    };
    if wash.saying != Some(says) {
        // A different sentence on the same sheet: walking down the panel
        // re-inks it and does not rebuild it. The ground starts again only
        // when there was no slip a frame ago, which is the bubble having
        // actually opened.
        if wash.saying.is_none() {
            wash.ground = 0.0;
        }
        wash.saying = Some(says);
        wash.ink = 0.0;
        wash.wipe = 0.0;
        wash.dried = 0.0;
    }

    if prefs.is_some_and(|p| p.all().reduce_motion) {
        *wash = SlipWash {
            saying: Some(says),
            ..SlipWash::done()
        };
    } else if wash.moving() {
        let ease = |rate: f32| 1.0 - (-rate * time.delta_secs()).exp();
        let feel = ease(crate::ambience::FEEL_RATE);
        wash.ground = towards(wash.ground, feel);
        if wash.ground >= INK_STARTS {
            wash.ink = towards(wash.ink, feel);
            // Linear, and therefore the one of the four that arrives exactly.
            wash.wipe = (wash.wipe + time.delta_secs() / across).min(1.0);
            wash.dried = towards(wash.dried, ease(DRY_RATE));
        }
    } else {
        // Every node is spawned at rest, so a finished slip needs nothing
        // written to it — including on the frame the pointer rebuilds it.
        return;
    }

    for (w, mut node, mut fill, mut transform) in &mut shapes {
        match w.part {
            Part::Ground => {
                *fill = BackgroundColor(w.base.with_alpha(w.base.alpha() * wash.ground));
                transform.translation = Val2::px(0.0, -SLIP_LIFT * (1.0 - wash.ground));
            }
            Part::Veil => node.top = Val::Percent(wash.wipe * 100.0),
            _ => {}
        }
    }
    for (w, ink, image) in &mut paints {
        match w.part {
            Part::Grain => {
                if let Some(mut image) = image {
                    image.color = w.base.with_alpha(w.base.alpha() * wash.ground);
                }
            }
            Part::Heading => {
                if let Some(mut ink) = ink {
                    *ink = TextColor(w.base.with_alpha(w.base.alpha() * wash.ground));
                }
            }
            Part::Ink => {
                if let Some(mut ink) = ink {
                    let wet = w.base.mix(&palette::PARCHMENT, WET * (1.0 - wash.dried));
                    *ink = TextColor(wet.with_alpha(w.base.alpha() * wash.ink));
                }
            }
            Part::Ground | Part::Veil => {}
        }
    }
}

/// The slip, as a node under the preview's card.
///
/// Spawned at rest; [`wash_the_slip_in`] is what writes it on.
pub(super) fn spawn(
    commands: &mut Commands,
    slip: &Slip,
    runs: Vec<Vec<Piece>>,
    width: f32,
    window_h: f32,
    fonts: &UiFonts,
    sheets: Option<&UiSheets>,
) -> Entity {
    let pen = Pen {
        says: slip.says,
        across: across(lines(&runs, width)),
    };
    let root = commands
        .spawn((
            Node {
                width: px(width),
                max_height: px(window_h * SLIP_MAX_H),
                margin: UiRect::top(px(SLIP_GAP)),
                padding: UiRect::all(px(SLIP_PAD)),
                flex_direction: FlexDirection::Column,
                row_gap: px(SLIP_ROW_GAP),
                border_radius: sheet_radius(),
                // The cap above is a cut, so it has to cut: without this a
                // sheet taller than the cap simply overflows it.
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT),
            pen.mark(Part::Ground, palette::PARCHMENT),
            // The whole bubble already ignores the pointer; this is a
            // *sibling* of the card's frame, so the recursive insert over
            // there does not reach it — and a panel that took the hover from
            // the row that opened it would close itself.
            Pickable::IGNORE,
        ))
        .id();
    if let Some(sheets) = sheets {
        commands
            .entity(root)
            .with_child((sheet_surface(sheets), pen.mark(Part::Grain, Color::WHITE)));
    }
    let heading = heading(commands, slip, pen, fonts);
    commands.entity(root).add_child(heading);
    let page = page(commands, runs, pen, fonts);
    commands.entity(root).add_child(page);
    root
}

/// The two facts every node of one slip carries.
///
/// A small `Copy` struct rather than a closure, so the builders below can be
/// separate functions: the alternative is one `spawn` of a hundred and
/// thirty lines, which is what this replaced.
#[derive(Clone, Copy)]
struct Pen {
    says: ObjectId,
    across: f32,
}

impl Pen {
    const fn mark(self, part: Part, base: Color) -> Washing {
        Washing {
            part,
            base,
            says: self.says,
            across: self.across,
        }
    }
}

/// What kind of thing this is and whose — the stack row's own subtitle, on
/// the sheet instead of under the picture.
fn heading(commands: &mut Commands, slip: &Slip, pen: Pen, fonts: &UiFonts) -> Entity {
    let line = commands
        .spawn((
            Text::default(),
            tf(fonts, SLIP_HEAD_PT),
            TextColor(palette::SLIP_SOFT),
            pen.mark(Part::Heading, palette::SLIP_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(kind) = &slip.kind {
        let span = commands
            .spawn((
                TextSpan::new(format!("{kind} — ")),
                tf(fonts, SLIP_HEAD_PT),
                TextColor(palette::SLIP_SOFT),
                pen.mark(Part::Heading, palette::SLIP_SOFT),
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    // The seat in the slant, and only the seat — the same split the stack
    // row's subtitle makes, because it is the same sentence.
    let seat = commands
        .spawn((
            TextSpan::new(slip.seat.clone()),
            tf_italic(fonts, SLIP_HEAD_PT),
            TextColor(palette::SLIP_SOFT),
            pen.mark(Part::Heading, palette::SLIP_SOFT),
        ))
        .id();
    commands.entity(line).add_child(seat);
    line
}

/// The printed body, paragraph by paragraph, and the wipe over all of it.
///
/// One box, so the wipe covers the writing and not the heading, and so its
/// own soft edge is clipped at the sheet's margin rather than spilling onto
/// it — and one [`Text`] per paragraph inside that box, because a card
/// prints paragraphs and one flat run of spans loses where they ended: a
/// planeswalker's three loyalty abilities came out as
/// `…oben auf deine Bibliothek.−1: Schicke…`.
fn page(commands: &mut Commands, runs: Vec<Vec<Piece>>, pen: Pen, fonts: &UiFonts) -> Entity {
    let page = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(SLIP_PARA_GAP),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for para in runs {
        let sentence = commands
            .spawn((
                Text::default(),
                super::tf_serif(fonts, SLIP_PT, super::INK_WEIGHT),
                bevy::text::LineHeight::Px(SLIP_LINE),
                TextColor(palette::SLIP_INK),
                pen.mark(Part::Ink, palette::SLIP_INK),
                Pickable::IGNORE,
            ))
            .id();
        // The halo, and only where it belongs: `bleed` answers `None` under
        // 12 px, where a second coloured copy of a stem *is* the stem.
        if let Some(halo) = super::bleed(SLIP_PT * super::SERIF_SCALE) {
            commands.entity(sentence).insert(halo);
        }
        for piece in para {
            let ink = if piece.reminder {
                palette::SLIP_ASIDE
            } else {
                palette::SLIP_INK
            };
            let span = commands
                .spawn((
                    TextSpan::new(piece.text),
                    if piece.mark {
                        crate::manaui::mana_tf(fonts, SLIP_PT * SLIP_MARK)
                    } else if piece.reminder {
                        super::tf_serif_italic(fonts, SLIP_PT, super::INK_WEIGHT)
                    } else {
                        super::tf_serif(fonts, SLIP_PT, super::INK_WEIGHT)
                    },
                    TextColor(ink),
                    pen.mark(Part::Ink, ink),
                ))
                .id();
            commands.entity(sentence).add_child(span);
        }
        commands.entity(page).add_child(sentence);
    }

    // Only the wipe's `top` moves: the fade is in the gradient's *pixel*
    // stops, so it stays the same thickness whatever the sheet's height is
    // and the system writes one number a frame. It is one veil over the
    // whole page and not one per paragraph, because the ink is meant to
    // arrive down the sheet in one movement.
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: percent(100),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::px(palette::PARCHMENT.with_alpha(0.0), 0.0),
                ColorStop::px(palette::PARCHMENT, WIPE_SOFT),
                ColorStop::percent(palette::PARCHMENT, 100.0),
            ])),
            pen.mark(Part::Veil, palette::PARCHMENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(page).add_child(veil);
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::Prefs;

    /// One paragraph of one plain run, which is what most of these want.
    fn para(text: &str) -> Vec<Piece> {
        vec![piece(text)]
    }

    fn piece(text: &str) -> Piece {
        Piece {
            text: text.to_string(),
            mark: false,
            reminder: false,
        }
    }

    // ---- what the slip says, and when it says nothing --------------------

    /// A printing with one sentence on it, filed where the fixtures put
    /// theirs.
    fn one_sentence() -> crate::cardtext::CardTexts {
        use baylee_client_core::card_face::{CardTextEntry, FaceText};
        crate::cardtext::CardTexts::filed(
            baylee_core::ids::PrintRef::new(7),
            CardTextEntry {
                scryfall_id: "abc".to_string(),
                lang: "de".to_string(),
                faces: vec![FaceText {
                    name: "Ondu-Kleriker".to_string(),
                    english_name: "Ondu Cleric".to_string(),
                    type_line: "Kreatur".to_string(),
                    oracle_text: "Ziehe eine Karte.".to_string(),
                    mana_cost: String::new(),
                }],
            },
        )
    }

    /// A table with one permanent, one ability of that permanent on the
    /// stack, and one card in hand.
    fn a_table() -> (baylee_client_core::BoardModel, PlayerView) {
        use baylee_client_core::board::Openings;
        use baylee_client_core::test_support::{ViewBuilder, printed, token};
        let mut ability = token(30, 0, "Ondu Cleric", 0, 0);
        ability.card = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: ObjectId::new(7, 0),
            ability: None,
            text: Some(baylee_view::StackText {
                face: 0,
                line: 0,
                of: 1,
            }),
        });
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(7, 0, "Ondu Cleric", 7)])
            .with_stack(vec![ability])
            .with_hand(vec![("Ondu Cleric", 7, 50)])
            .build();
        let board = baylee_client_core::BoardModel::from_view(
            &view,
            Openings::none(),
            |_| 800.0,
            crate::cardart::registry(),
        );
        (board, view)
    }

    fn ctx<'a>(
        texts: &'a crate::cardtext::CardTexts,
        mode: &'a crate::face::FaceMode,
        settings: &'a crate::settings::ClientSettings,
        view: &'a PlayerView,
    ) -> FaceCtx<'a> {
        FaceCtx {
            texts,
            mode,
            settings,
            view: Some(view),
        }
    }

    /// The sentence the row cut, whole — and the heading that says whose
    /// ability it is.
    #[test]
    fn an_ability_on_the_stack_gets_its_own_sentence() {
        let texts = one_sentence();
        let mode = crate::face::FaceMode::default();
        let settings = crate::settings::ClientSettings::default();
        let (board, view) = a_table();
        let statics = baylee_client_core::test_support::statics(8);
        let slip = says(
            &board,
            &view,
            &statics,
            Lang::De,
            &ctx(&texts, &mode, &settings, &view),
            Some(ObjectId::new(30, 0)),
        )
        .expect("the ability is on the stack and its printing is filed");
        assert_eq!(slip.says, ObjectId::new(30, 0));
        assert!(
            slip.kind.is_some(),
            "an ability names the permanent it came from"
        );
        assert_eq!(
            super::runs(&slip)
                .iter()
                .flatten()
                .map(|piece| piece.text.as_str())
                .collect::<String>(),
            "Ziehe eine Karte.",
            "the player's own printing, in the player's own language"
        );
    }

    /// A planeswalker's three loyalty abilities are three paragraphs, and
    /// this is the defect the first live shot showed: drawn as one flat run
    /// of spans they came out as `…oben auf deine Bibliothek.−1: Schicke…`,
    /// with the second ability starting in the middle of the first one's
    /// last line. The printing holds the boundary in a newline and
    /// [`baylee_client_core::card_face::split_blocks`] does not carry it, so
    /// the split happens before it is asked.
    #[test]
    fn a_walker_on_the_stack_keeps_its_abilities_apart() {
        use baylee_client_core::board::Openings;
        use baylee_client_core::card_face::{CardTextEntry, FaceText};
        use baylee_client_core::test_support::{ViewBuilder, printed};
        let texts = crate::cardtext::CardTexts::filed(
            baylee_core::ids::PrintRef::new(9),
            CardTextEntry {
                scryfall_id: "def".to_string(),
                lang: "de".to_string(),
                faces: vec![FaceText {
                    name: "Aminatou, die Schicksalswenderin".to_string(),
                    english_name: "Aminatou, the Fateshifter".to_string(),
                    type_line: "Legendärer Planeswalker — Aminatou".to_string(),
                    oracle_text: "+1: Ziehe eine Karte. Lege dann eine Karte \
                        aus deiner Hand oben auf deine Bibliothek.\n\
                        −1: Schicke eine bleibende Karte, die du kontrollierst, \
                        ins Exil. Bringe sie dann unter der Kontrolle ihres \
                        Besitzers ins Spiel zurück.\n\
                        −6: Wähle bis zu einen Spieler."
                        .to_string(),
                    mana_cost: String::new(),
                }],
            },
        );
        let mut spell = printed(31, 0, "Aminatou, the Fateshifter", 9);
        spell.stack_item = Some(baylee_view::StackItem::Spell);
        let view = ViewBuilder::new(2).with_stack(vec![spell]).build();
        let board = baylee_client_core::BoardModel::from_view(
            &view,
            Openings::none(),
            |_| 800.0,
            crate::cardart::registry(),
        );
        let mode = crate::face::FaceMode::default();
        let settings = crate::settings::ClientSettings::default();
        let statics = baylee_client_core::test_support::statics(8);
        let slip = super::says(
            &board,
            &view,
            &statics,
            Lang::De,
            &ctx(&texts, &mode, &settings, &view),
            Some(ObjectId::new(31, 0)),
        )
        .expect("a spell on the stack, with its printing filed");
        let runs = super::runs(&slip);
        assert_eq!(runs.len(), 3, "three loyalty abilities, three paragraphs");
        let said: Vec<String> = runs
            .iter()
            .map(|para| para.iter().map(|piece| piece.text.as_str()).collect())
            .collect();
        assert!(
            said[0].ends_with("oben auf deine Bibliothek."),
            "the first paragraph ends where the card's line ends: {:?}",
            said[0]
        );
        assert!(
            said[1].starts_with("−1: Schicke"),
            "the second begins at its own cost: {:?}",
            said[1]
        );
        assert!(
            said.iter().all(|para| !para.contains(".−")),
            "no paragraph carries the next one's cost: {said:?}"
        );
    }

    /// The text face and the slip are the same words, so only one of them is
    /// ever drawn — and the switch is the player's *choice*, not "a face came
    /// back", which is also how a card whose art is still in flight is drawn
    /// and is the ordinary case offline.
    #[test]
    fn the_text_face_is_the_slip_and_is_not_drawn_twice() {
        let texts = one_sentence();
        let mode = crate::face::FaceMode::default();
        let settings = crate::settings::ClientSettings {
            prefer_text_view: true,
            ..Default::default()
        };
        let (board, view) = a_table();
        let statics = baylee_client_core::test_support::statics(8);
        assert!(
            says(
                &board,
                &view,
                &statics,
                Lang::De,
                &ctx(&texts, &mode, &settings, &view),
                Some(ObjectId::new(30, 0)),
            )
            .is_none()
        );
    }

    /// Everything that is not on the stack. The hover preview opens over
    /// hands, permanents, piles and browser rows too, and none of those is a
    /// thing about to resolve.
    #[test]
    fn nothing_that_is_not_on_the_stack_gets_a_slip() {
        let texts = one_sentence();
        let mode = crate::face::FaceMode::default();
        let settings = crate::settings::ClientSettings::default();
        let (board, view) = a_table();
        let statics = baylee_client_core::test_support::statics(8);
        let faces = ctx(&texts, &mode, &settings, &view);
        for (what, id) in [
            ("a card in hand", ObjectId::new(50, 0)),
            ("the permanent the ability came from", ObjectId::new(7, 0)),
            ("nothing at all", ObjectId::new(99, 0)),
        ] {
            assert!(
                says(&board, &view, &statics, Lang::De, &faces, Some(id)).is_none(),
                "{what} is not about to resolve"
            );
        }
        assert!(says(&board, &view, &statics, Lang::De, &faces, None).is_none());
    }

    /// The wipe is paced by the reading and clamped at both ends: a one-line
    /// trigger must not flicker and a nine-line spell must not be a wait.
    #[test]
    fn the_wipe_is_clamped_at_both_ends() {
        assert!((across(1) - WIPE_MIN).abs() < 1e-6, "one line is the floor");
        assert!(
            (across(40) - WIPE_MAX).abs() < 1e-6,
            "a card nobody has printed is the ceiling"
        );
        let five = across(5);
        assert!(
            five > WIPE_MIN && five < WIPE_MAX,
            "five lines is between them: {five}"
        );
    }

    /// The longest ability in the pool is 113 characters, and the brief says
    /// three lines of a sheet this wide. The estimate has to agree, because
    /// it is what the wipe's duration is taken from.
    #[test]
    fn a_long_ability_is_three_lines_of_a_card_wide_sheet() {
        let long = "x".repeat(113);
        assert_eq!(lines(&[para(&long)], 308.0), 3);
        assert_eq!(lines(&[para("Draw a card.")], 308.0), 1);
        assert_eq!(lines(&[], 308.0), 1, "an empty page is still one line");
    }

    /// A sheet is taller when it carries a heading, and a longer sentence
    /// makes it taller still. Both halves are what the placement reads.
    #[test]
    fn the_sheet_grows_with_what_is_on_it() {
        let one = [para("Draw a card.")];
        let bare = height(&one, 308.0, false);
        let headed = height(&one, 308.0, true);
        assert!(
            headed > bare,
            "a heading costs a row: {headed} against {bare}"
        );
        let long = [para(&"x".repeat(113))];
        assert!(
            height(&long, 308.0, false) > bare,
            "three lines are taller than one"
        );
        // And a second paragraph costs its own line *and* the gap between
        // them, which is the arithmetic the placement above the sheet
        // leaves room for.
        let two = [para("Draw a card."), para("Draw a card.")];
        let apart = height(&two, 308.0, false);
        assert!(
            (apart - bare - SLIP_LINE - SLIP_PARA_GAP).abs() < 0.01,
            "{apart} is {bare} plus a line and a gap"
        );
    }

    /// The builder's half of the same claim: a paragraph is a `Text` of its
    /// own, so the line break between two of them is the layout's and not a
    /// character anyone had to print. The model keeping the paragraphs
    /// apart buys nothing if they are all poured into one line afterwards.
    #[test]
    fn each_paragraph_is_drawn_as_a_line_of_its_own() {
        let mut app = App::new();
        let fonts = UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        };
        let slip = Slip {
            says: ObjectId::new(31, 0),
            kind: None,
            seat: "Du".to_string(),
            blocks: Vec::new(),
        };
        let runs = vec![
            para("+1: Ziehe eine Karte."),
            para("−1: Schicke sie ins Exil."),
        ];
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let root = {
            let mut commands = Commands::new(&mut queue, app.world());
            spawn(&mut commands, &slip, runs, 308.0, 1052.0, &fonts, None)
        };
        queue.apply(app.world_mut());
        // The sheet is heading, then page; the page is one line per
        // paragraph, then the veil over all of them.
        let page = *app
            .world()
            .entity(root)
            .get::<Children>()
            .expect("a sheet has a heading and a page")
            .last()
            .expect("the page is the last of them");
        let kids: Vec<Entity> = app
            .world()
            .entity(page)
            .get::<Children>()
            .expect("the page has its lines")
            .iter()
            .collect();
        let lines: Vec<Entity> = kids
            .iter()
            .copied()
            .filter(|e| app.world().entity(*e).contains::<Text>())
            .collect();
        assert_eq!(lines.len(), 2, "two paragraphs, two lines");
        for line in lines {
            let spans = app
                .world()
                .entity(line)
                .get::<Children>()
                .expect("a line carries its runs");
            assert_eq!(spans.len(), 1, "and each line carries only its own");
        }
        assert_eq!(
            app.world()
                .entity(page)
                .get::<Node>()
                .expect("the page is a node")
                .row_gap,
            px(SLIP_PARA_GAP),
            "the gap between them is the one the height was measured with"
        );
    }

    // ---- the system, actually run ---------------------------------------

    /// An app with the wash in it and nothing else.
    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Prefs>()
            .init_resource::<SlipWash>()
            .add_systems(Update, wash_the_slip_in);
        app
    }

    /// A slip's three interesting nodes, spawned at rest the way the builder
    /// spawns them.
    fn a_slip(app: &mut App, says: ObjectId) -> (Entity, Entity, Entity) {
        let ground = app
            .world_mut()
            .spawn((
                Node::default(),
                BackgroundColor(palette::PARCHMENT),
                Washing {
                    part: Part::Ground,
                    base: palette::PARCHMENT,
                    says,
                    across: WIPE_MIN,
                },
            ))
            .id();
        let ink = app
            .world_mut()
            .spawn((
                Node::default(),
                TextColor(palette::SLIP_INK),
                Washing {
                    part: Part::Ink,
                    base: palette::SLIP_INK,
                    says,
                    across: WIPE_MIN,
                },
            ))
            .id();
        let veil = app
            .world_mut()
            .spawn((
                Node {
                    top: percent(100),
                    ..default()
                },
                Washing {
                    part: Part::Veil,
                    base: palette::PARCHMENT,
                    says,
                    across: WIPE_MIN,
                },
            ))
            .id();
        (ground, ink, veil)
    }

    /// A sixtieth of a second, the frame this client is tuned against.
    fn a_frame(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.update();
    }

    fn ink_alpha(app: &App, node: Entity) -> f32 {
        app.world()
            .entity(node)
            .get::<TextColor>()
            .unwrap()
            .0
            .alpha()
    }

    /// The whole reason the ordering in `add_present_systems` is spelled out:
    /// the nodes are spawned written, so the first frame has to *unwrite*
    /// them. A slip that arrived at full strength would be a flash.
    #[test]
    fn the_first_frame_takes_the_ink_back_off() {
        let mut app = harness();
        let (ground, ink, _) = a_slip(&mut app, ObjectId::new(1, 0));
        a_frame(&mut app);
        assert!(
            ink_alpha(&app, ink) < 1e-6,
            "the ink waits for the ground: {}",
            ink_alpha(&app, ink)
        );
        let fill = app
            .world()
            .entity(ground)
            .get::<BackgroundColor>()
            .unwrap()
            .0
            .alpha();
        assert!(fill > 0.0 && fill < 1.0, "the ground is on its way: {fill}");
    }

    /// And it finishes. A wash that never landed would leave the sentence
    /// permanently pale, which is worse than no animation at all.
    #[test]
    fn the_slip_comes_to_rest() {
        let mut app = harness();
        let (ground, ink, veil) = a_slip(&mut app, ObjectId::new(1, 0));
        for _ in 0..120 {
            a_frame(&mut app);
        }
        assert!((ink_alpha(&app, ink) - palette::SLIP_INK.alpha()).abs() < 1e-4);
        let colour = app.world().entity(ink).get::<TextColor>().unwrap().0;
        let (dry, wet) = (palette::SLIP_INK.to_srgba().red, colour.to_srgba().red);
        assert!(
            (wet - dry).abs() < 1e-3,
            "the ink dried: {wet} against {dry}"
        );
        assert_eq!(
            app.world()
                .entity(ground)
                .get::<UiTransform>()
                .unwrap()
                .translation,
            Val2::px(0.0, 0.0),
            "the sheet came to rest"
        );
        assert_eq!(
            app.world().entity(veil).get::<Node>().unwrap().top,
            Val::Percent(100.0),
            "the wipe crossed the page"
        );
    }

    /// Wet ink is lighter than dry ink, and it is the *same* ink — the mix is
    /// what lets the aside dry alongside the rules text without a second
    /// colour being named for it.
    #[test]
    fn ink_lands_wet_and_dries() {
        let mut app = harness();
        let (_, ink, _) = a_slip(&mut app, ObjectId::new(1, 0));
        // Far enough in that the ink is on the page and not yet dry.
        for _ in 0..8 {
            a_frame(&mut app);
        }
        let wet = app
            .world()
            .entity(ink)
            .get::<TextColor>()
            .unwrap()
            .0
            .to_srgba()
            .red;
        assert!(
            wet > palette::SLIP_INK.to_srgba().red,
            "it lands lighter than it dries: {wet}"
        );
    }

    /// `reduce_motion` is a setting about movement, not about information:
    /// the slip is simply *there*, written, on the first frame.
    #[test]
    fn a_player_who_asked_for_no_motion_gets_the_sentence_at_once() {
        let mut app = harness();
        app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = true;
        let (_, ink, veil) = a_slip(&mut app, ObjectId::new(1, 0));
        a_frame(&mut app);
        assert!((ink_alpha(&app, ink) - palette::SLIP_INK.alpha()).abs() < 1e-6);
        assert_eq!(
            app.world().entity(veil).get::<Node>().unwrap().top,
            Val::Percent(100.0)
        );
    }

    /// Walking down the panel re-inks the sheet and does not rebuild it. The
    /// ground is the expensive half of the movement, and a sheet that faded
    /// in again under a pointer travelling three rows would read as a
    /// flicker.
    #[test]
    fn the_next_row_re_inks_the_sheet_it_is_already_on() {
        let mut app = harness();
        let first = ObjectId::new(1, 0);
        let (ground, ..) = a_slip(&mut app, first);
        for _ in 0..60 {
            a_frame(&mut app);
        }
        let fill = |app: &App| {
            app.world()
                .entity(ground)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .alpha()
        };
        assert!((fill(&app) - 1.0).abs() < 1e-4, "the sheet is there");

        // The pointer moves to the row below: same sheet, new sentence.
        let mut says = app.world_mut().query::<&mut Washing>();
        for mut w in says.iter_mut(app.world_mut()) {
            w.says = ObjectId::new(2, 0);
        }
        a_frame(&mut app);
        assert!(
            (fill(&app) - 1.0).abs() < 1e-4,
            "the sheet stayed: {}",
            fill(&app)
        );
        assert!(
            app.world().resource::<SlipWash>().ink < 1.0,
            "and the ink started again"
        );
    }

    /// The bubble closing takes the wash with it. A progress kept would greet
    /// the next hover with a slip that was already written.
    #[test]
    fn a_closed_bubble_leaves_nothing_behind() {
        let mut app = harness();
        let (ground, ..) = a_slip(&mut app, ObjectId::new(1, 0));
        for _ in 0..30 {
            a_frame(&mut app);
        }
        app.world_mut().entity_mut(ground).despawn();
        let mut rest = app.world_mut().query::<Entity>().iter(app.world()).count();
        assert!(rest > 0, "the ink and the veil are still there");
        for entity in app
            .world_mut()
            .query_filtered::<Entity, With<Washing>>()
            .iter(app.world())
            .collect::<Vec<_>>()
        {
            app.world_mut().entity_mut(entity).despawn();
        }
        rest = app
            .world_mut()
            .query_filtered::<Entity, With<Washing>>()
            .iter(app.world())
            .count();
        assert_eq!(rest, 0);
        a_frame(&mut app);
        let wash = app.world().resource::<SlipWash>();
        assert_eq!(wash.saying, None);
        assert!(wash.ground.abs() < f32::EPSILON, "and it starts again");
    }
}
