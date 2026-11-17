//! Semantic color-coded rich prose for Tower item descriptions.
//!
//! Highlights Life (`+X LeP` / `X life` in warm emerald, life-loss/pay-life in
//! warm rose), Damage (`X Schaden` / `X damage` in warm rose), Gold (`+X Gold`
//! in coin gold), Charge counters (`Ladungsmarken` / `charge counters` in arcane
//! cyan), per-turn activation limits in warm amber, and standalone numbers/stats
//! in crisp bold ivory, while rendering embedded `{mana}` symbols as real pips.

use crate::hud::{UiFonts, tf, tf_bold};
use crate::shellkit::{surfaces::whole_pixels, tokens};
use baylee_client_core::manapip::{self, Pip, Segment};
use bevy::prelude::*;
use bevy::text::TextSpan;

/// Semantic tone for a span of item rules text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Body,
    Number,
    Life,
    Damage,
    Gold,
    Charge,
    Limit,
}

impl Tone {
    const fn color(self) -> Color {
        match self {
            Self::Body => tokens::INK,
            Self::Number => Color::srgb(0.98, 0.93, 0.78),
            Self::Life => Color::srgb(0.46, 0.92, 0.62),
            Self::Damage => Color::srgb(0.98, 0.48, 0.52),
            Self::Gold => Color::srgb(1.0, 0.85, 0.40),
            Self::Charge => Color::srgb(0.48, 0.88, 0.98),
            Self::Limit => Color::srgb(0.96, 0.78, 0.44),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StyledSpan<'a> {
    pub text: &'a str,
    pub tone: Tone,
    pub bold: bool,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '+' | '-' | '/' | '×')
}

fn has_digit(word: &str) -> bool {
    word.bytes().any(|b| b.is_ascii_digit())
}

fn is_number_word(lower: &str) -> bool {
    has_digit(lower)
        || matches!(
            lower,
            "a" | "one"
                | "two"
                | "three"
                | "four"
                | "five"
                | "ein"
                | "eine"
                | "einen"
                | "zwei"
                | "drei"
                | "vier"
                | "fünf"
        )
}

fn is_life_noun(lower: &str) -> bool {
    matches!(
        lower,
        "lep" | "start-lep" | "lebenspunkte" | "lebenspunkt" | "life"
    )
}

fn is_damage_noun(lower: &str) -> bool {
    matches!(lower, "schaden" | "kampfschaden" | "damage")
}

fn is_gold_noun(lower: &str) -> bool {
    lower == "gold"
}

fn is_charge_noun(lower: &str) -> bool {
    matches!(
        lower,
        "ladungsmarken" | "ladungsmarke" | "ladung" | "charge"
    )
}

fn find_limit_tail(text: &str) -> Option<usize> {
    for marker in ["Nur einmal pro Zug", "Aktiviere dies nur", "Activate only "] {
        if let Some(pos) = text.find(marker) {
            return Some(pos);
        }
    }
    None
}

/// Splits `text` into styled spans whose concatenation is byte-for-byte identical
/// to `text`, preserving every space, umlaut, and punctuation mark.
pub(crate) fn stylize(text: &str) -> Vec<StyledSpan<'_>> {
    if text.is_empty() {
        return Vec::new();
    }
    if let Some(limit_at) = find_limit_tail(text) {
        let mut out = if limit_at > 0 {
            stylize_body(text, 0, limit_at)
        } else {
            Vec::new()
        };
        push_range(&mut out, text, limit_at, text.len(), Tone::Limit, false);
        return out;
    }
    stylize_body(text, 0, text.len())
}

fn stylize_body(base: &str, start_off: usize, end_off: usize) -> Vec<StyledSpan<'_>> {
    let text = &base[start_off..end_off];
    // Partition `text` into `(w_start..w_end, w_end..s_end)` byte offsets in `base`.
    let mut tokens: Vec<(usize, usize, usize)> = Vec::new();
    let mut idx = 0;
    let bytes = text.len();
    let mut leading_end = 0;
    if let Some((first_w, _)) = text.char_indices().find(|&(_, c)| is_word_char(c)) {
        if first_w > 0 {
            leading_end = first_w;
            idx = first_w;
        }
    } else {
        return vec![StyledSpan {
            text,
            tone: Tone::Body,
            bold: false,
        }];
    }

    while idx < bytes {
        let rest = &text[idx..];
        let w_end = rest
            .char_indices()
            .find(|&(_, c)| !is_word_char(c))
            .map_or(bytes, |(off, _)| idx + off);
        let after = &text[w_end..];
        let s_end = after
            .char_indices()
            .find(|&(_, c)| is_word_char(c))
            .map_or(bytes, |(off, _)| w_end + off);
        tokens.push((start_off + idx, start_off + w_end, start_off + s_end));
        idx = s_end;
    }

    let tones = classify_tokens(base, &tokens);

    let mut out = Vec::new();
    if leading_end > 0 {
        push_range(
            &mut out,
            base,
            start_off,
            start_off + leading_end,
            Tone::Body,
            false,
        );
    }
    for (idx, &(ws, we, se)) in tokens.iter().enumerate() {
        let (wtone, wbold) = tones[idx];
        push_range(&mut out, base, ws, we, wtone, wbold);
        if se > we {
            let sep = &base[we..se];
            let sep_tone = if sep == " "
                && wtone != Tone::Body
                && tones.get(idx + 1).is_some_and(|&(nt, _)| nt == wtone)
            {
                wtone
            } else {
                Tone::Body
            };
            push_range(&mut out, base, we, se, sep_tone, false);
        }
    }
    out
}

fn classify_tokens(base: &str, tokens: &[(usize, usize, usize)]) -> Vec<(Tone, bool)> {
    let lowers: Vec<String> = tokens
        .iter()
        .map(|&(ws, we, _)| base[ws..we].to_lowercase())
        .collect();
    let n = tokens.len();
    let mut tones = vec![(Tone::Body, false); n];
    let mut clause_drains_life = false;
    let mut i = 0;
    while i < n {
        let low = lowers[i].as_str();
        if matches!(low, "verliert" | "bezahle" | "loses" | "pay") {
            clause_drains_life = true;
        }
        let next1 = lowers.get(i + 1).map(String::as_str);
        let next2 = lowers.get(i + 2).map(String::as_str);
        if let Some(step) = classify_pair(low, next1, next2, clause_drains_life, &mut tones[i..]) {
            i += step;
            continue;
        }
        let (_, we, se) = tokens[i];
        if base[we..se]
            .chars()
            .any(|c| matches!(c, '.' | ':' | ';' | '·'))
        {
            clause_drains_life = false;
        }
        i += 1;
    }
    tones
}

fn classify_pair(
    low: &str,
    next1: Option<&str>,
    next2: Option<&str>,
    drains_life: bool,
    out: &mut [(Tone, bool)],
) -> Option<usize> {
    let life_tone = if drains_life {
        Tone::Damage
    } else {
        Tone::Life
    };
    if is_number_word(low) {
        if next1.is_some_and(is_gold_noun) {
            out[0] = (Tone::Gold, has_digit(low));
            out[1] = (Tone::Gold, false);
            return Some(2);
        }
        if next1.is_some_and(is_damage_noun) {
            out[0] = (Tone::Damage, has_digit(low));
            out[1] = (Tone::Damage, false);
            return Some(2);
        }
        if next1.is_some_and(is_life_noun) || (next1 == Some("starting") && next2 == Some("life")) {
            out[0] = (life_tone, has_digit(low));
            out[1] = (life_tone, false);
            if next1 == Some("starting") && next2 == Some("life") {
                out[2] = (life_tone, false);
                return Some(3);
            }
            return Some(2);
        }
        if next1.is_some_and(is_charge_noun) {
            out[0] = (Tone::Charge, has_digit(low));
            out[1] = (Tone::Charge, false);
            if next1 == Some("charge") && matches!(next2, Some("counter" | "counters")) {
                out[2] = (Tone::Charge, false);
                return Some(3);
            }
            return Some(2);
        }
        if has_digit(low) {
            out[0] = (Tone::Number, true);
        }
        return None;
    }
    if is_gold_noun(low) {
        out[0] = (Tone::Gold, false);
    } else if is_damage_noun(low) {
        out[0] = (Tone::Damage, false);
    } else if is_life_noun(low) {
        out[0] = (life_tone, false);
    } else if is_charge_noun(low) {
        out[0] = (Tone::Charge, false);
        if low == "charge" && matches!(next1, Some("counter" | "counters")) {
            out[1] = (Tone::Charge, false);
            return Some(2);
        }
    }
    None
}

fn push_range<'a>(
    out: &mut Vec<StyledSpan<'a>>,
    base: &'a str,
    from: usize,
    to: usize,
    tone: Tone,
    bold: bool,
) {
    if from >= to {
        return;
    }
    if let Some(last) = out.last_mut()
        && last.tone == tone
        && last.bold == bold
    {
        let base_addr = base.as_ptr() as usize;
        let last_from = last.text.as_ptr() as usize - base_addr;
        let last_to = last_from + last.text.len();
        if last_to == from {
            last.text = &base[last_from..to];
            return;
        }
    }
    out.push(StyledSpan {
        text: &base[from..to],
        tone,
        bold,
    });
}

fn spawn_styled_text(commands: &mut Commands, fonts: &UiFonts, text: &str, size: f32) -> Entity {
    let px_size = whole_pixels(size);
    let spans = stylize(text);
    let Some((first, rest)) = spans.split_first() else {
        return commands
            .spawn((
                Text::default(),
                tf(fonts, px_size),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
    };
    let root = commands
        .spawn((
            Text::new(first.text),
            if first.bold {
                tf_bold(fonts, px_size)
            } else {
                tf(fonts, px_size)
            },
            TextColor(first.tone.color()),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    for span in rest {
        let child = commands
            .spawn((
                TextSpan::new(span.text),
                if span.bold {
                    tf_bold(fonts, px_size)
                } else {
                    tf(fonts, px_size)
                },
                TextColor(span.tone.color()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).add_child(child);
    }
    root
}

/// Spawns one rich item-effect line with semantic color-coding AND real `{mana}`
/// pip discs (using a single wrapped `Text` node when the line has no mana pips
/// so word-boundary wrapping stays exact).
pub(crate) fn spawn_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    line: &str,
    size: f32,
) -> Entity {
    let segments = manapip::segments(line);
    if !segments.iter().any(|seg| matches!(seg, Segment::Symbol(_))) {
        return spawn_styled_text(commands, fonts, line, size);
    }
    let gap = crate::manaui::air(size);
    let marks = size * 1.08;
    let row = commands
        .spawn((
            Node {
                column_gap: px(gap),
                row_gap: px(gap),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut taken = 0usize;
    for (at, segment) in segments.iter().enumerate() {
        let child = match segment {
            Segment::Text(words) => {
                let words = &words[taken.min(words.len())..];
                taken = 0;
                if words.is_empty() {
                    continue;
                }
                spawn_styled_text(commands, fonts, words, size)
            }
            Segment::Symbol(pip) => {
                let disc = match pip {
                    Pip::Loyalty(loy) => {
                        crate::manaui::spawn_loyalty_badge(
                            commands,
                            fonts,
                            *loy,
                            marks,
                            tokens::INK,
                            tokens::OPAQUE,
                        )
                        .root
                    }
                    other => crate::manaui::spawn_pip(commands, fonts, *other, marks),
                };
                let glued = match segments.get(at + 1) {
                    Some(Segment::Text(next)) => manapip::clings(next),
                    _ => 0,
                };
                taken = glued;
                if glued == 0 {
                    disc
                } else if let Some(Segment::Text(next)) = segments.get(at + 1) {
                    let pair = commands
                        .spawn((
                            Node {
                                align_items: AlignItems::Center,
                                flex_wrap: FlexWrap::NoWrap,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .id();
                    let punct = spawn_styled_text(commands, fonts, &next[..glued], size);
                    commands.entity(pair).add_children(&[disc, punct]);
                    pair
                } else {
                    disc
                }
            }
        };
        commands.entity(row).add_child(child);
    }
    row
}
