//! Drawing the filter-string builder, in whichever register it is opened in.
//!
//! The model is `baylee_client_core::filterdialog`, and it holds everything
//! that is a decision: which rows there are, what each row's control is, what
//! a tap on one *means*. This file draws them and maps a tap back to the
//! [`Act`] the model already named. That split is why it is one file and not
//! two — the zone browser and the deck builder open the same builder over the
//! same language, and a second drawing of it would be a second set of
//! buttons to keep in step with the model.
//!
//! What differs between the two is a [`Register`]: six colours. The tray is
//! candle on dark oak and the lobby is teal on slate, and nothing else about
//! the panel changes — which is the test of whether the split was made in the
//! right place.
//!
//! Every button carries a [`FilterAct`], and **nothing** here calls a method
//! on the panel. Whoever owns the panel reads the component and hands the
//! `Act` to it, so a button that is drawn is a button that works: there is no
//! way to draw one and forget to wire it, which is the failure this client
//! has shipped before.

use bevy::prelude::*;

use baylee_client_core::cardquery::{Colors, Flag, Key, Op, Surface, Term, Value, render};
use baylee_client_core::filterdialog::{
    Act, Adding, ColorRule, Control, FLAGS, FilterPanel, FilterPart, OFFERED, SYMBOLS, Typing,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::textbuf::TextBuffer;

use crate::ambience::Feel;
use crate::hud::{UiFonts, glyph, icon_tf, palette, tf, tf_bold};

/// What one button in the builder means.
///
/// One component for every button on the panel, in both registers. The two
/// owners differ in *which panel* they hand it to and in nothing else.
#[derive(Component, Clone, Copy)]
pub(crate) struct FilterAct(pub(crate) Act);

/// The button at the foot that shuts the builder.
///
/// Its own marker and **not** an [`Act`], for the reason the gear is not one
/// either: the vocabulary says what the builder *means*, and whether the
/// panel is on screen at all is the question its container answers. A model
/// that could close itself would be a model that knew it was drawn.
#[derive(Component, Clone, Copy)]
pub(crate) struct FilterDone;

/// The six colours a register is.
///
/// A struct rather than two copies of the drawing, and six fields rather than
/// a whole theme: what a panel of controls needs is a ground, a control's
/// ground, a line, two inks and one colour that means *on*. Anything a
/// seventh field would buy is a second drawing wearing one name.
#[derive(Clone, Copy)]
pub(crate) struct Register {
    /// The panel's own fill.
    pub(crate) ground: Color,
    /// A control's fill at rest.
    pub(crate) control: Color,
    /// A border.
    pub(crate) line: Color,
    /// Text.
    pub(crate) ink: Color,
    /// Text that is an aside.
    pub(crate) soft: Color,
    /// What *on* looks like: a lit border, a caret, a chosen pip.
    pub(crate) lit: Color,
}

impl Register {
    /// The zone browser's: candle on dark oak.
    pub(crate) const TRAY: Self = Self {
        ground: palette::DIALOG,
        control: palette::DIALOG_LIT,
        line: palette::DIALOG_LINE,
        ink: palette::DIALOG_INK,
        soft: palette::DIALOG_SOFT,
        lit: palette::CANDLE,
    };

    /// The lobby's and the deck builder's: teal on slate.
    pub(crate) const LOBBY: Self = Self {
        ground: palette::PANEL,
        control: palette::PANEL_LIT,
        line: palette::DEAD,
        ink: palette::INK,
        soft: palette::MUTED,
        lit: palette::ACCENT,
    };
}

/// The height every control on the panel stands in.
const CTRL_H: f32 = 22.0;
/// The size a control's label is set at.
const LABEL: f32 = 10.5;
/// The size a row's own text box is set at.
const FIELD: f32 = 11.0;
/// The gap between controls in a row, and between rows.
const GAP: f32 = 5.0;

/// Draws the whole builder and answers with its root node.
///
/// `surface` is what the list under it can answer, and it reaches two places:
/// the flags offered on an `is:` row, and the warning under any row asking
/// something this list cannot check. Both come from the model — this only
/// chooses whether to draw them.
pub(crate) fn build(
    commands: &mut Commands,
    fonts: &UiFonts,
    panel: &FilterPanel,
    surface: Surface,
    lang: Lang,
    look: Register,
) -> Entity {
    let root = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(GAP),
                padding: UiRect::all(px(9)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(7)),
                ..default()
            },
            BackgroundColor(look.ground),
            BorderColor::all(look.line),
            Pickable::IGNORE,
        ))
        .id();

    let mut children = vec![heading(commands, fonts, lang, look)];
    let typing = panel.typing();
    for (at, part) in panel.parts().iter().enumerate() {
        let caret = typing.filter(|t| t.row() == at);
        children.push(match part {
            FilterPart::Row { negated, term } => row(
                commands,
                fonts,
                RowLook {
                    at,
                    negated: *negated,
                    term,
                    // The panel decides this, never the term on its own: it
                    // is the one place that also knows where the caret is.
                    control: panel.control(at).unwrap_or(Control::Text),
                    caret,
                    lang,
                    look,
                    surface,
                },
            ),
            FilterPart::Opaque(query) => as_typed(commands, fonts, at, &render(query), lang, look),
        });
    }
    if !panel.unanswerable(surface).is_empty() {
        children.push(note(
            commands,
            fonts,
            Phrase::FilterUnanswerable.text(lang),
            look,
        ));
    }
    children.push(adder(commands, fonts, panel.adding(), lang, look));
    children.push(foot(commands, fonts, lang, look));
    commands.entity(root).add_children(&children);
    root
}

/// The line over the rows, which names what the panel is.
fn heading(commands: &mut Commands, fonts: &UiFonts, lang: Lang, look: Register) -> Entity {
    label(
        commands,
        fonts,
        Phrase::FilterBuild.text(lang),
        LABEL,
        look.soft,
    )
}

/// What a row needs to draw itself.
struct RowLook<'a> {
    at: usize,
    negated: bool,
    term: &'a Term,
    control: Control,
    caret: Option<&'a Typing>,
    lang: Lang,
    look: Register,
    surface: Surface,
}

/// One condition: the minus, what it asks about, how, and the answer.
// One `match` over the five controls, read top to bottom. Splitting it into
// five functions would put the five three-line arms in five places and leave
// the reader walking between them to see what a row is made of.
#[allow(clippy::too_many_lines)]
fn row(commands: &mut Commands, fonts: &UiFonts, it: RowLook<'_>) -> Entity {
    let RowLook {
        at,
        negated,
        term,
        control,
        caret,
        lang,
        look,
        surface,
    } = it;
    let line = band(commands, None);
    let mut kids = vec![
        // The minus is a toggle and not a word, because it is the one control
        // whose meaning is the same in every row: *not this*.
        icon(
            commands,
            fonts,
            glyph::MINUS,
            Act::Negate(at, !negated),
            negated,
            look,
        ),
        label(
            commands,
            fonts,
            key_name(&term.key, lang),
            LABEL,
            if surface.answers(&term.key) {
                look.ink
            } else {
                look.soft
            },
        ),
    ];
    match control {
        Control::Colors => {
            kids.push(reading(commands, fonts, at, term, lang, look));
            kids.extend(pips(commands, fonts, at, &term.value, look));
        }
        Control::Number => {
            kids.push(comparison(commands, fonts, at, term.op, look));
            kids.push(field(commands, fonts, at, term, caret, look));
            kids.push(icon(
                commands,
                fonts,
                glyph::MINUS,
                Act::Bump(at, -1),
                false,
                look,
            ));
            kids.push(icon(
                commands,
                fonts,
                glyph::PLUS,
                Act::Bump(at, 1),
                false,
                look,
            ));
            // Only where the *key* reads a number. A colour count is drawn
            // with this control too, and `c:even` is not a search — it goes
            // through the same reader every other value does and comes back
            // an unreadable word.
            if Control::of(&term.key) == Control::Number {
                for (even, phrase) in [(true, Phrase::FilterEven), (false, Phrase::FilterOdd)] {
                    let on = term.value == Value::Parity(even);
                    kids.push(word(
                        commands,
                        fonts,
                        phrase.text(lang),
                        Act::Parity(at, (!on).then_some(even)),
                        on,
                        look,
                    ));
                }
            }
        }
        Control::Flag => {
            for (which, flag) in FLAGS.iter().enumerate() {
                if !surface.knows(*flag) {
                    continue;
                }
                let on = term.value == Value::Flag(*flag);
                kids.push(word(
                    commands,
                    fonts,
                    flag_name(*flag, lang),
                    Act::SetFlag(at, which),
                    on,
                    look,
                ));
            }
        }
        Control::Cost => {
            kids.push(field(commands, fonts, at, term, caret, look));
            for (which, symbol) in SYMBOLS.iter().enumerate() {
                kids.push(word(
                    commands,
                    fonts,
                    symbol,
                    Act::PushSymbol(at, which),
                    false,
                    look,
                ));
            }
            kids.push(icon(
                commands,
                fonts,
                glyph::BACKSPACE,
                Act::PopSymbol(at),
                false,
                look,
            ));
        }
        Control::Text => {
            kids.push(field(commands, fonts, at, term, caret, look));
        }
    }
    kids.push(icon(
        commands,
        fonts,
        glyph::CLOSE,
        Act::Remove(at),
        false,
        look,
    ));
    commands.entity(line).add_children(&kids);
    line
}

/// A branch no control stands for, shown as the player wrote it.
///
/// It is a *quotation* and not a control: the only thing offered is the ✕,
/// because `a or b` minus `a` is `b` — a different question — and a dialog
/// that let one half of an `or` be edited would be answering a question
/// nobody asked. `docs/client.md` §"Taking a filter string apart" has the
/// whole reasoning.
fn as_typed(
    commands: &mut Commands,
    fonts: &UiFonts,
    at: usize,
    written: &str,
    lang: Lang,
    look: Register,
) -> Entity {
    let line = band(commands, None);
    let kids = [
        label(commands, fonts, written, FIELD, look.ink),
        label(
            commands,
            fonts,
            Phrase::FilterAsTyped.text(lang),
            LABEL,
            look.soft,
        ),
        icon(commands, fonts, glyph::CLOSE, Act::Remove(at), false, look),
    ];
    commands.entity(line).add_children(&kids);
    line
}

/// The row's own text box: what it says, and the caret when it holds it.
fn field(
    commands: &mut Commands,
    fonts: &UiFonts,
    at: usize,
    term: &Term,
    caret: Option<&Typing>,
    look: Register,
) -> Entity {
    let box_node = commands
        .spawn((
            FilterAct(Act::Edit(at)),
            Button,
            Node {
                min_width: px(64),
                height: px(CTRL_H - 4.0),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(look.ground),
            BorderColor::all(if caret.is_some() { look.lit } else { look.line }),
            Feel::new(look.ground),
        ))
        .id();
    let kids = match caret {
        // Typing draws head, caret, selection and tail, the same way the
        // tray's own search box does — one door onto `TextBuffer::segments`
        // rather than a second reading of where a caret is.
        Some(typing) => runs(commands, fonts, typing.field(), look),
        None => vec![label(
            commands,
            fonts,
            &term.value.written(),
            FIELD,
            look.ink,
        )],
    };
    commands.entity(box_node).add_children(&kids);
    box_node
}

/// Head, caret, selection, tail.
fn runs(
    commands: &mut Commands,
    fonts: &UiFonts,
    field: &TextBuffer,
    look: Register,
) -> Vec<Entity> {
    let seg = field.segments();
    let caret_at = usize::from(seg.caret_after_selection) + 1;
    let mut out = Vec::new();
    for (i, (text, selected)) in [(seg.head, false), (seg.selected, true), (seg.tail, false)]
        .into_iter()
        .enumerate()
    {
        if i == caret_at {
            out.push(
                commands
                    .spawn((
                        Node {
                            width: px(1),
                            // Bevy lays a text node out at 1.2 times its font
                            // size, so a shorter bar would stand lower than
                            // the letters beside it and read as a fault.
                            height: px(FIELD * 1.2),
                            margin: UiRect::horizontal(px(-0.5)),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(look.lit),
                        Pickable::IGNORE,
                    ))
                    .id(),
            );
        }
        if text.is_empty() {
            continue;
        }
        let run = label(commands, fonts, text, FIELD, look.ink);
        if selected {
            commands
                .entity(run)
                .insert(BackgroundColor(palette::SELECTION));
        }
        out.push(run);
    }
    out
}

/// The three sentences a colour row may be asking.
///
/// Sentences and not operators, which is the model's own decision: a player
/// choosing between `c:` and `c=` is choosing between *at least* and
/// *exactly*, and `ColorRule` is what maps each back to the spelling its key
/// uses. An operator with no reading — `c!=w` — lights none of the three and
/// is left exactly as typed.
fn reading(
    commands: &mut Commands,
    fonts: &UiFonts,
    at: usize,
    term: &Term,
    lang: Lang,
    look: Register,
) -> Entity {
    let held = ColorRule::of(&term.key, term.op);
    let strip = band(commands, Some(look.line));
    let kids: Vec<Entity> = [
        (ColorRule::AtLeast, Phrase::FilterAtLeast),
        (ColorRule::Exactly, Phrase::FilterExactly),
        (ColorRule::AtMost, Phrase::FilterAtMost),
    ]
    .into_iter()
    .map(|(rule, phrase)| {
        word(
            commands,
            fonts,
            phrase.text(lang),
            Act::SetRule(at, rule),
            held == Some(rule),
            look,
        )
    })
    .collect();
    commands.entity(strip).add_children(&kids);
    strip
}

/// The six pips a colour row is answered with.
fn pips(
    commands: &mut Commands,
    fonts: &UiFonts,
    at: usize,
    value: &Value,
    look: Register,
) -> Vec<Entity> {
    let held = match value {
        Value::Colors(colors) => *colors,
        _ => Colors::default(),
    };
    let colourless = matches!(value, Value::Colors(c) if c.letters().is_empty());
    "WUBRGC"
        .chars()
        .map(|letter| {
            let on = if letter == 'C' {
                colourless
            } else {
                held.holds(Colors::of_letters(&letter.to_string()))
            };
            word(
                commands,
                fonts,
                &letter.to_string(),
                Act::Colour(at, letter),
                on,
                look,
            )
        })
        .collect()
}

/// The six comparisons a number row may use.
fn comparison(
    commands: &mut Commands,
    fonts: &UiFonts,
    at: usize,
    held: Op,
    look: Register,
) -> Entity {
    let strip = band(commands, Some(look.line));
    let kids: Vec<Entity> = [Op::Lt, Op::Le, Op::Eq, Op::Ge, Op::Gt, Op::Ne]
        .into_iter()
        .map(|op| {
            word(
                commands,
                fonts,
                op.render(),
                Act::SetOp(at, op),
                // A bare colon on a number key *is* `=`, so a row a player
                // typed as `mv:3` lights the same button as `mv=3` rather
                // than lighting none of the six.
                op == if held == Op::Colon { Op::Eq } else { held },
                look,
            )
        })
        .collect();
    commands.entity(strip).add_children(&kids);
    strip
}

/// The add button, and whichever of its two steps is unfolded.
///
/// Two steps and not one list of thirteen keys: the first asks what *kind* of
/// condition, and the second offers the keys of that kind. The grouping is
/// `Control::of` over the same list the rows are added from, so a key cannot
/// go missing from the menu by being left out of a second one.
fn adder(
    commands: &mut Commands,
    fonts: &UiFonts,
    adding: Adding,
    lang: Lang,
    look: Register,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // The button folds the menu away again, so the way out of it is the way
    // in — a player who opened it by mistake does not have to find a second
    // control to close it with.
    let step = if adding == Adding::Closed {
        Adding::Kinds
    } else {
        Adding::Closed
    };
    let opener = band(commands, None);
    let button = word(
        commands,
        fonts,
        Phrase::FilterAdd.text(lang),
        Act::AddStep(step),
        adding != Adding::Closed,
        look,
    );
    commands.entity(opener).add_child(button);
    let mut kids = vec![opener];

    match adding {
        Adding::Closed => {}
        Adding::Kinds => {
            let line = band(commands, None);
            let buttons: Vec<Entity> = [
                (Control::Text, Phrase::FilterKindText),
                (Control::Colors, Phrase::FilterKindColor),
                (Control::Number, Phrase::FilterKindNumber),
                (Control::Flag, Phrase::FilterKindFlag),
                (Control::Cost, Phrase::FilterKindCost),
            ]
            .into_iter()
            .map(|(kind, phrase)| {
                word(
                    commands,
                    fonts,
                    phrase.text(lang),
                    Act::AddStep(Adding::Keys(kind)),
                    false,
                    look,
                )
            })
            .collect();
            commands.entity(line).add_children(&buttons);
            kids.push(line);
        }
        Adding::Keys(kind) => {
            let line = band(commands, None);
            let buttons: Vec<Entity> = OFFERED
                .iter()
                .enumerate()
                .filter(|(_, key)| Control::of(key) == kind)
                .map(|(which, key)| {
                    word(
                        commands,
                        fonts,
                        key_name(key, lang),
                        Act::Add(which),
                        false,
                        look,
                    )
                })
                .collect();
            commands.entity(line).add_children(&buttons);
            kids.push(line);
        }
    }
    commands.entity(column).add_children(&kids);
    column
}

/// The two words at the foot: empty it, and close it.
fn foot(commands: &mut Commands, fonts: &UiFonts, lang: Lang, look: Register) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                column_gap: px(GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let done = word(
        commands,
        fonts,
        Phrase::FilterDone.text(lang),
        Act::StopTyping,
        false,
        look,
    );
    // The same button is also the way out of the panel, which is why it
    // carries both: giving a row's caret back is the builder's business and
    // closing the panel is its container's, and a player pressing *Done*
    // means both at once.
    commands.entity(done).insert(FilterDone);
    let kids = [
        word(
            commands,
            fonts,
            Phrase::FilterClear.text(lang),
            Act::Clear,
            false,
            look,
        ),
        done,
    ];
    commands.entity(line).add_children(&kids);
    line
}

/// One line under the rows, when any of them asks what this list cannot
/// answer.
///
/// One line and not one per row, because it is the same fact each time — and
/// the row itself already says it, by having its key set in the soft ink.
fn note(commands: &mut Commands, fonts: &UiFonts, text: &str, look: Register) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(glyph::INFO.to_string()),
            icon_tf(fonts, LABEL),
            TextColor(look.lit),
            Pickable::IGNORE,
        ))
        .id();
    let words = label(commands, fonts, text, LABEL, look.soft);
    commands.entity(line).add_children(&[mark, words]);
    line
}

// --------------------------------------------------------------- the pieces

/// A row of controls, optionally fenced as one segmented strip.
///
/// The fence is what makes a set of mutually exclusive answers read as one
/// control: the three colour readings and the six comparisons are each *one*
/// question, and six separate buttons in a row of eleven is a row nobody can
/// see the shape of.
fn band(commands: &mut Commands, fence: Option<Color>) -> Entity {
    let mut node = Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(if fence.is_some() { 0.0 } else { GAP }),
        min_height: px(CTRL_H),
        flex_wrap: FlexWrap::Wrap,
        row_gap: px(3.0),
        ..default()
    };
    let mut entity = commands.spawn_empty();
    if let Some(line) = fence {
        node.border = UiRect::all(px(1));
        node.border_radius = BorderRadius::all(px(4));
        node.overflow = Overflow::clip();
        node.flex_wrap = FlexWrap::NoWrap;
        entity.insert(BorderColor::all(line));
    }
    entity.insert((node, Pickable::IGNORE));
    entity.id()
}

/// A word that can be pressed, lit when what it stands for is chosen.
fn word(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    act: Act,
    on: bool,
    look: Register,
) -> Entity {
    let ground = if on { look.lit } else { look.control };
    let ink = if on { look.ground } else { look.ink };
    let button = commands
        .spawn((
            FilterAct(act),
            Button,
            Node {
                height: px(CTRL_H - 4.0),
                padding: UiRect::horizontal(px(7)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(if on { look.lit } else { look.line }),
            Feel::new(ground),
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(text.to_string()),
            tf_bold(fonts, LABEL),
            TextColor(ink),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(words);
    button
}

/// The same, as a glyph from the icon face.
fn icon(
    commands: &mut Commands,
    fonts: &UiFonts,
    mark: char,
    act: Act,
    on: bool,
    look: Register,
) -> Entity {
    let ground = if on { look.lit } else { look.control };
    let ink = if on { look.ground } else { look.soft };
    let button = commands
        .spawn((
            FilterAct(act),
            Button,
            Node {
                width: px(CTRL_H - 4.0),
                height: px(CTRL_H - 4.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(if on { look.lit } else { look.line }),
            Feel::new(ground),
        ))
        .id();
    let glyph_node = commands
        .spawn((
            Text::new(mark.to_string()),
            icon_tf(fonts, LABEL),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(glyph_node);
    button
}

/// A line of writing that is not a control.
fn label(commands: &mut Commands, fonts: &UiFonts, text: &str, size: f32, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(text.to_string()),
            tf(fonts, size),
            TextColor(ink),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Pickable::IGNORE,
        ))
        .id()
}

// ------------------------------------------------------------- the words

/// What a key is called, in the player's language.
///
/// A key the language does not know is shown *as the player wrote it* — the
/// one place this panel says something a translation must not touch, because
/// the word is theirs and not the interface's.
fn key_name(key: &Key, lang: Lang) -> &str {
    match key {
        Key::Loose => Phrase::FilterKeyLoose.text(lang),
        Key::Name => Phrase::FilterKeyName.text(lang),
        Key::ExactName => Phrase::FilterKeyExact.text(lang),
        Key::Oracle => Phrase::FilterKeyOracle.text(lang),
        Key::Type => Phrase::FilterKeyType.text(lang),
        Key::Color => Phrase::FilterKeyColor.text(lang),
        Key::Identity => Phrase::FilterKeyIdentity.text(lang),
        Key::Mana => Phrase::FilterKeyMana.text(lang),
        Key::ManaValue => Phrase::FilterKeyManaValue.text(lang),
        Key::Power => Phrase::FilterKeyPower.text(lang),
        Key::Toughness => Phrase::FilterKeyToughness.text(lang),
        Key::Loyalty => Phrase::FilterKeyLoyalty.text(lang),
        Key::Is => Phrase::FilterKindFlag.text(lang),
        Key::Unknown(word) => word,
    }
}

/// What a flag is called.
fn flag_name(flag: Flag, lang: Lang) -> &'static str {
    match flag {
        Flag::Playable => Phrase::FlagPlayable.text(lang),
        Flag::Partial => Phrase::CoveragePartial.text(lang),
        Flag::Stub => Phrase::CoverageStub.text(lang),
        Flag::Commander => Phrase::FlagCommander.text(lang),
        Flag::Basic => Phrase::FlagBasic.text(lang),
        Flag::Dfc => Phrase::FlagDfc.text(lang),
        Flag::Token => Phrase::IsToken.text(lang),
    }
}
