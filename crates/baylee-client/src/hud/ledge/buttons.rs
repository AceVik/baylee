//! The columns, sentences and answer buttons the rows are drawn with.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;

/// Which of the three columns a node is, and where its centre goes.
pub(super) enum Side {
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
pub(super) fn column_node(side: Side) -> impl Bundle {
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
        Side::Right => {
            node.right = px(EDGE);
            node.justify_content = JustifyContent::End;
            // One button today, and the step is kept at a button's rather
            // than a sentence's against the day there are two again: this
            // column used to hold the pair that is now behind the burger, and
            // [`RIGHT_RESERVED`] is measured with this gap in it.
            node.column_gap = px(baylee_client_core::ledge::BUTTON_GAP);
        }
        // Full width and centred, so the question stands on the **window's**
        // middle — which is the middle of this seat's own mat. When `arrange`
        // has had to slide it off centre, the slide is paid for out of one
        // side's padding ([`mid_padding`]), which is the same arithmetic the
        // drawer stands on.
        Side::Mid(mid_x, window_w) => {
            node.left = px(0);
            node.right = px(0);
            node.justify_content = JustifyContent::Center;
            node.padding = mid_padding(mid_x, window_w);
        }
    }
    // The middle lies over the other two across the whole width, so without
    // this a draw offer and a concession would be dead or alive depending on
    // the order the three were spawned in — "a label swallows the hover", one
    // level up. The buttons inside it are pickable in their own right.
    (node, Pickable::IGNORE)
}

/// The right column: the door to the game menu.
///
/// It was a row of pills in the window's top-right corner, over the felt,
/// with the priority hold's chip beside it. The hold went to the middle
/// (§4.4, and it is the answer to a question the middle is asking), and the
/// two ways out came here, which is where they were always about to be: the
/// shelf has three columns, and leaving the game belongs to no seat and to no
/// question.
///
/// They are not drawn here any more. The owner asked on 19.09.2026 for them
/// to go behind one button — *"Aus den zwei Buttons rechts wird ein Burger
/// Menü"* — so this column holds the burger and [`menu`] holds the pair. The
/// column is what changed, not the argument for it: a way out still belongs
/// to no seat and to no question, and it is still at this end of the shelf.
///
/// **After `GameOver` the column is empty**, which it also was before, and
/// for a reason the burger does not escape. There is nothing left to concede
/// and nobody left to offer a draw to, and `DuelSet::Input` does not run in
/// `Finished` — so a button left standing under the end screen would warm
/// under the pointer and answer nothing, which is exactly what the pair used
/// to do up in the corner. [`menu::sync_menu`] shuts the panel on the same
/// reading.
///
/// Neither entry in that menu wears a keycap and neither is going to: a draw
/// offer is not a thing to press by accident, and a concession is that twice
/// over. The burger does not wear one either, for a third reason — it opens a
/// place rather than doing anything, and `Esc` already closes it.
pub(super) fn ways_out(
    commands: &mut Commands,
    fonts: &UiFonts,
    column: Entity,
    revision: &LedgeRevision,
) {
    if revision.over {
        return;
    }
    let burger = menu::burger(commands, fonts, revision.menu_open);
    commands.entity(column).add_child(burger);
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
/// `size` because the drawer writes in this voice too and writes quieter: a
/// hint about where to click is not as loud as the question it is under.
pub(super) fn sentence(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    ink: Color,
) -> Entity {
    // Payment questions carry the same printed symbols as ability costs.
    // Keep ordinary prose in one text layout; a symbol-bearing question uses
    // the shared Mana-font renderer, including symbols inside an aside.
    if baylee_client_core::manapip::segments(text)
        .iter()
        .any(|part| matches!(part, baylee_client_core::manapip::Segment::Symbol(_)))
    {
        let line = commands
            .spawn((
                Node {
                    flex_wrap: bevy::ui::FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for (run, aside) in baylee_client_core::prose::bracketed(text) {
            let span = crate::manaui::spawn_rich_in(
                commands,
                fonts,
                run,
                size,
                if aside { palette::LEDGE_SOFT } else { ink },
                tf_italic,
            );
            commands.entity(line).add_child(span);
        }
        return line;
    }
    let line = commands
        .spawn((
            Text::default(),
            tf_italic(fonts, size),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    for (run, aside) in baylee_client_core::prose::bracketed(text) {
        let span = commands
            .spawn((
                TextSpan::new(run.to_string()),
                tf_italic(fonts, size),
                TextColor(if aside { palette::LEDGE_SOFT } else { ink }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    line
}

/// How loud an answer is.
///
/// Five, and the last two are the right column's, which is why they arrived
/// with it in §10.2 step 5: a concession waiting for its second press is the
/// loudest thing this shelf ever says, and a draw the engine would refuse is
/// the quietest. [`palette::LEDGE_DEAD`] came in for the empty pool's em dash
/// and says exactly the same thing on a button — a place where something
/// would be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Weight {
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
    /// A concession that has been armed and is waiting for its second press.
    ///
    /// The one weight louder than the candle, and the only one on the shelf
    /// that is not an invitation. Rejected: a concession drawn in this colour
    /// at rest — then the second stage says nothing new — and a concession
    /// drawn as a ghost, which is too quiet for the hardest thing the button
    /// does.
    Danger,
    /// A control that is drawn because its place is reserved, and that cannot
    /// be pressed: a draw offer outside this seat's own priority.
    ///
    /// Not a greyed-out fill but **no fill at all**, so it reads as a place
    /// something would be rather than as a button somebody has switched off.
    /// It carries no [`Feel`] and no `MenuButton`, and takes
    /// `Pickable::IGNORE`: a control the pointer warms and the press ignores
    /// is a lie one frame long.
    Dead,
}

impl Weight {
    /// Fill, border, ink.
    pub(super) const fn colours(self) -> (Color, Color, Color) {
        match self {
            // `DIALOG` on `CANDLE` is 7.39 : 1. Light ink on the candle is
            // 1.86 : 1 and is forbidden outright — see the palette.
            Self::Candle => (palette::CANDLE, palette::CANDLE, palette::DIALOG),
            // A dark inset key with a champagne edge, subordinate to the
            // candle-filled default without disappearing into the dock.
            Self::Secondary => (palette::DOCK_GROUND, palette::DOCK_EDGE, palette::DOCK_INK),
            Self::Ghost => (Color::NONE, Color::NONE, palette::DIALOG_INK),
            // `DIALOG` on `DANGER` is the second pair this shelf is held to
            // — §3.2 measures it at 6.11 : 1, and it is the same dark ink the
            // candle carries, because the two loud buttons are one register.
            Self::Danger => (palette::DANGER, palette::DANGER, palette::DIALOG),
            // A border and nothing behind it: the box is where the button
            // would be, and `LEDGE_DEAD` is the ink the empty pool writes its
            // em dash in.
            Self::Dead => (Color::NONE, palette::DIALOG_LINE, palette::LEDGE_DEAD),
        }
    }

    /// How it answers the pointer, or nothing where it must not answer at all.
    ///
    /// `Feel::new` shades a colour towards white and **keeps its alpha**, so
    /// a ghost resting at `Color::NONE` would be lifted to a brighter nothing
    /// and never answer the pointer at all. Its hot end is therefore stated.
    ///
    /// [`Dead`](Self::Dead) is the one weight with no answer: a button that
    /// warms under the pointer and does nothing when pressed is worse than a
    /// button that is plainly not there.
    pub(super) fn feel(self) -> Option<Feel> {
        Some(match self {
            Self::Candle => Feel::new(palette::CANDLE),
            Self::Secondary => Feel::new(palette::DOCK_GROUND),
            Self::Ghost => Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
            Self::Danger => Feel::new(palette::DANGER),
            Self::Dead => return None,
        })
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
    ///
    /// The right column's two wear none at all — §4.3 refuses a key for
    /// either, a draw offer because it is not a thing to press by accident
    /// and a concession for the same reason twice over — so their arm here is
    /// the quiet one and is never reached by a drawing.
    const fn cap_colours(self) -> (Color, Color, Color) {
        match self {
            Self::Candle => (palette::DIALOG, palette::DIALOG, palette::CANDLE),
            Self::Secondary | Self::Ghost | Self::Danger | Self::Dead => {
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
pub(in crate::hud) fn answer(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    weight: Weight,
    cap: Option<&str>,
) -> Entity {
    let id = answer_sized(commands, fonts, label, weight, cap, BUTTON_H, LABEL_PT);
    commands.entity(id).entry::<Node>().and_modify(|mut n| {
        n.height = px(BUTTON_H);
        n.min_height = Val::Auto;
        n.flex_shrink = 1.0;
    });
    id
}

/// The game menu's button treatment at a screen-appropriate touch size.
pub(crate) fn answer_sized(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    weight: Weight,
    cap: Option<&str>,
    height: f32,
    font_size: f32,
) -> Entity {
    let (fill, edge, ink) = weight.colours();
    let button = commands
        .spawn((
            Node {
                min_height: px(height),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(CAP_GAP),
                padding: UiRect::axes(px(BUTTON_PAD_X), px(BUTTON_PAD_Y)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .id();
    // A dead control gets neither, which is the whole of what makes it dead:
    // no warmth under the pointer, and no press to be swallowed by a box that
    // was never going to answer it.
    if let Some(feel) = weight.feel() {
        commands.entity(button).insert(feel);
    } else {
        commands.entity(button).insert(Pickable::IGNORE);
    }
    // A machined key, not a floating pill. Both bevels stay inside the
    // existing border box and neither adds a target or changes measurement.
    if matches!(weight, Weight::Candle | Weight::Secondary | Weight::Danger) {
        for (top, colour) in [
            (true, palette::DOCK_INK.with_alpha(0.18)),
            (false, palette::DOCK_GROUND.with_alpha(0.65)),
        ] {
            let bevel = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(3),
                        right: px(3),
                        top: if top { px(1) } else { Val::Auto },
                        bottom: if top { Val::Auto } else { px(1) },
                        height: px(1),
                        ..default()
                    },
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(button).add_child(bevel);
        }
    }
    if let Some(legend) = cap {
        let (cap_fill, cap_edge, cap_ink) = weight.cap_colours();
        let key = keycap(commands, fonts, legend, cap_fill, cap_ink, cap_edge, CAP_PT);
        commands.entity(button).add_child(key);
    }
    let words = if baylee_client_core::manapip::segments(label)
        .iter()
        .any(|run| matches!(run, baylee_client_core::manapip::Segment::Symbol(_)))
    {
        let words = crate::manaui::spawn_rich_in(commands, fonts, label, font_size, ink, tf_bold);
        keep_label_on_one_line(commands, words);
        words
    } else {
        commands
            .spawn((
                Text::new(label),
                tf_bold(fonts, font_size),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id()
    };
    commands.entity(button).add_child(words);
    button
}

/// The armed deed as a pair of buttons: the deed itself, and the way back.
///
/// Two buttons and no sentence between them, because the first one *is* the
/// sentence — a line reading "Play this card" beside a button called "Send"
/// says the same thing twice and leaves a player to work out which half is
/// the button. It is the one state that takes the question off the shelf.
pub(super) fn armed_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    prefs: &crate::prefs::Prefs,
    row: Entity,
    words: &super::overlay::ArmedWords,
) {
    let caps = keys_for(prefs, &[], true, false);
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
    keep_label_on_one_line(commands, node);
    commands.entity(button).add_child(node);
}

/// Fixed-height controls must measure the whole symbol run, not its widest pip.
fn keep_label_on_one_line(commands: &mut Commands, label: Entity) {
    commands
        .entity(label)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_wrap = bevy::ui::FlexWrap::NoWrap;
            node.flex_shrink = 0.0;
        });
}

/// How wide a keycap's box is, legend and air together.
///
/// [`KEYCAP_SIDE`] is a floor and not a width: a cap grows with its legend,
/// so `F6` sits in the 20-pixel square and `⇧Tab` does not.
pub(super) fn cap_width(legend: &str) -> f32 {
    (CAP_PT * KEYCAP_SIDE).max(super::text_width(legend, CAP_PT, true) + 2.0 * CAP_PT * 0.45)
}

/// How wide the middle column will be, before it is laid out.
///
/// The estimate [`baylee_client_core::ledge::arrange`] is fed, and the reason
/// [`super::text_width`] exists — `bevy_ui` measures text during layout, and
/// this is a decision the layout depends on.
pub(super) fn mid_width(
    sentence: Option<&str>,
    clock: Clock,
    answers: &[(Says, String)],
    caps: &[Option<String>],
) -> f32 {
    let buttons: f32 = answers
        .iter()
        .enumerate()
        .map(|(i, (says, label))| {
            let cap = caps
                .get(i)
                .and_then(Option::as_deref)
                .map_or(0.0, |c| cap_width(c) + CAP_GAP);
            let icon = if *says == Says::Command(super::MenuAction::ToggleGrantedActions) {
                13.0 + CAP_GAP
            } else {
                0.0
            };
            cap + icon + super::text_width(label, LABEL_PT, true) + 2.0 * BUTTON_PAD_X
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let gaps = baylee_client_core::ledge::BUTTON_GAP * answers.len().saturating_sub(1) as f32;
    let words = sentence.map_or(0.0, |text| {
        super::text_width(text, SENTENCE_PT, false) + baylee_client_core::ledge::SENTENCE_GAP
    });
    let clock = match clock {
        Clock::None => 0.0,
        Clock::Beside => clock_width(),
        Clock::InButton => button_clock_width() + CAP_GAP,
    };
    clock + words + buttons + gaps
}
