//! What is left of the phase rail: how a step of a turn is drawn, the two
//! eased lights that said where the game was, and the combat lines.
//!
//! The rail itself is gone. It was a strip fifty-four pixels tall under a
//! seat-tab strip of fifty-six, and between them they took a hundred and ten
//! off the top of every window to say twelve things about one turn — while
//! every seat's own mat had a shelf on it doing nothing. [`super::seatbar`]
//! is where a turn is drawn now, once per seat, on the seat's own ground.
//!
//! [`row_visual`] survived the move: a step's glyph and its three letters are
//! the same wherever a step is drawn, and the seat bar reads them from here.
//! So did the two eased lights. The **step light** is the bar's baton: the
//! bars draw the step the table's pace shows ([`pace_the_steps`], held a
//! while per step so a person can follow the turn going round), and
//! [`glow_the_steps`] fades the gold off the step left and onto the step
//! shown. It keeps the light by seat and step ([`super::StepLights`]), off
//! the tiles, because the bars are rebuilt whole with every step: a light
//! eased from zero at spawn arrived on the new tile and died at once on the
//! old (29.10.2026). The **hinge-light** is not written, so nothing spawns a
//! [`Designation`] outside a test and [`flash_the_designation`] still runs
//! over an empty query; a flash anchored to an entity restarts every time
//! the pointer moves, which is what it keeps the last value against. Their
//! tests spawn the markers by hand.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::combat::{Combat, LineEnd};

/// Icon and the short rail label for a rail row.
pub(super) fn row_visual(row: RailRow) -> (char, &'static str) {
    let index = match row {
        RailRow::Untap => 0,
        RailRow::Upkeep => 1,
        RailRow::Draw => 2,
        RailRow::Main1 => 3,
        RailRow::CombatBegin => 4,
        RailRow::Attackers => 5,
        RailRow::Blockers => 6,
        RailRow::Damage => 7,
        RailRow::CombatEnd => 8,
        RailRow::Main2 => 9,
        RailRow::EndStep => 10,
        RailRow::Cleanup => 11,
    };
    (
        baylee_client_core::tableicons::PHASES[index],
        [
            "UNT", "UPK", "DRW", "M1", "CBT", "ATK", "BLK", "DMG", "EOC", "M2", "END", "CLN",
        ][index],
    )
}

/// The designation the flash last saw, and when it changed.
///
/// A resource rather than a field on the block, because the block is a new
/// entity after every HUD rebuild and the whole point is to survive one. It
/// starts at `None`, which is also what a game with no designation has — so
/// the first block to appear flashes, and its arrival is the announcement.
#[derive(Resource, Default)]
pub struct DesignationFlash {
    /// What was on screen when the clock was last stamped.
    seen: Option<baylee_view::DayNight>,
    /// `Time::elapsed_secs` at the change.
    at: f32,
}

/// How fast the flash on a designation change decays, per second. About a
/// twentieth left after a second — a beat, not an animation.
const FLASH_DECAY: f32 = 3.0;

/// Marks a change of designation with a brief light around the block.
///
/// It writes the border and the shadow and leaves the background alone,
/// because `Feel` owns that one, and two systems writing one component is a
/// fight the frame order decides. The decay is anchored to the change
/// rather than to the entity's birth, so a HUD rebuilt mid-decay — which the
/// pointer does constantly — picks the flash up where it was instead of
/// starting it again.
pub fn flash_the_designation(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut state: ResMut<DesignationFlash>,
    mut blocks: Query<(&Designation, &mut BorderColor, &mut BoxShadow)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let now = time.elapsed_secs();
    for (block, mut border, mut shadow) in &mut blocks {
        if state.seen != Some(block.0) {
            state.seen = Some(block.0);
            state.at = now;
        }
        let raw = if still {
            0.0
        } else {
            (-FLASH_DECAY * (now - state.at)).exp()
        };
        let flash = if raw < 1e-3 { 0.0 } else { raw };
        let tone = match block.0 {
            baylee_view::DayNight::Day => palette::PARCHMENT,
            baylee_view::DayNight::Night => palette::INK,
        };
        let mut edge = tone.to_srgba();
        edge.alpha = 0.55 * flash;
        let next_border = BorderColor::all(Color::from(edge));
        if *border != next_border {
            *border = next_border;
        }
        if let Some(first) = shadow.first() {
            let mut glow = tone.to_srgba();
            glow.alpha = 0.55 * flash;
            let next_color = Color::from(glow);
            let next_blur = Val::Px(NOW_GLOW * flash);
            let next_spread = Val::Px(NOW_SPREAD * flash);
            if (first.color != next_color
                || first.blur_radius != next_blur
                || first.spread_radius != next_spread)
                && let Some(first) = shadow.first_mut()
            {
                first.color = next_color;
                first.blur_radius = next_blur;
                first.spread_radius = next_spread;
            }
        }
    }
}

/// How fast the step light comes onto a tile and leaves the one before, per
/// second: about nine tenths of the way in 0.65 s, so the light is seen to
/// pass from step to step at the pace `phasepace::STEP_HOLD` holds each one
/// (owner, 29.10.2026: slower, smooth, followable); it leaves at twice the
/// rate. It used to arrive in a quarter of a second, and leave in none: the
/// old tile went with the tree.
const LIGHT_RATE: f32 = 3.5;

/// How far the light around the current step's button reaches, and how far it
/// stands off the button's own edge.
const NOW_GLOW: f32 = 10.0;
const NOW_SPREAD: f32 = 2.0;

/// Follows the view's step with the table's pace ([`super::StepPace`]):
/// each step a view stood in is shown for a while before the next, briefly
/// while this seat must answer, and reduced motion goes straight to the
/// step.
/// Writes only when what is shown changes; a new game, or a replay that went
/// back, starts over.
pub fn pace_the_steps(
    time: Res<Time>,
    duel: Option<Res<Duel>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut pace: ResMut<super::StepPace>,
) {
    let Some(view) = duel.as_ref().and_then(|d| d.view.as_ref()) else {
        return;
    };
    let seen = baylee_client_core::phasepace::Shown {
        turn: view.turn,
        active: view.active,
        phase: view.phase,
        step: view.step,
    };
    // A replay that went back, or jumped turns ahead, is a new place, not a
    // turn going round: shown at once.
    if pace
        .0
        .shown()
        .is_some_and(|shown| shown.turn > seen.turn || seen.turn > shown.turn + 1)
    {
        pace.0.reset();
    }
    let asked = view.awaiting == Some(view.seat);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let quiet = pace.bypass_change_detection();
    if quiet.0.follow(time.elapsed_secs(), seen, asked, still) {
        pace.set_changed();
    }
}

/// The view the seat bars are built from: the view, standing in the step,
/// phase, turn and active seat the table's pace shows. `None` when the pace
/// shows the view's own.
#[must_use]
pub(super) fn paced(view: &PlayerView, pace: &super::StepPace) -> Option<PlayerView> {
    let shown = pace.0.shown()?;
    let own = shown.turn == view.turn
        && shown.active == view.active
        && shown.phase == view.phase
        && shown.step == view.step;
    (!own).then(|| PlayerView {
        turn: shown.turn,
        active: shown.active,
        phase: shown.phase,
        step: shown.step,
        ..view.clone()
    })
}

/// Lights the step the table shows, and lets the light go from the step it
/// left.
///
/// The bars are rebuilt whole with every step, so a light that lived on the
/// tile was born at nothing on the new step and died at once on the old.
/// The light lives here instead ([`super::StepLights`], by seat and step),
/// eases up on the step shown and down on every other, and every part that
/// wears it is written from it, a rebuilt one on the frame it appears: the
/// tile's glow and frame ([`super::StepGlow`]), the gold under its glyph
/// ([`super::StepFill`]) and its ink ([`super::StepInk`]), so the gold
/// passes from tile to tile instead of jumping. Only a step that grants
/// priority is lit, as only those were gold (CR 502.4, CR 514.3a). Every
/// write is compared first, and once every light has settled nothing is
/// written at all.
///
/// The tile's own background is left alone, because [`Feel`] owns that one
/// and two systems writing the same component is a fight the frame order
/// decides; the gold is a child's.
#[allow(clippy::type_complexity)] // a tile's parts, each written apart
pub fn glow_the_steps(
    time: Res<Time>,
    pace: Res<super::StepPace>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut lights: ResMut<super::StepLights>,
    mut tiles: Query<(&super::StepGlow, &mut BorderColor, &mut BoxShadow)>,
    mut fills: Query<(&super::StepFill, &mut BackgroundColor)>,
    mut inks: Query<
        (
            &super::StepInk,
            Option<&mut TextColor>,
            Option<&mut BackgroundColor>,
        ),
        Without<super::StepFill>,
    >,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let shown = pace
        .0
        .shown()
        .filter(|s| s.row().grants_priority())
        .map(|s| (s.active, s.row()));
    let ease = |rate: f32| {
        if still {
            1.0
        } else {
            1.0 - (-rate * time.delta_secs()).exp()
        }
    };
    // The light leaves twice as fast as it comes: at the pace's hold a step
    // left still glowed when the next was lit, and the gold trailed three
    // tiles behind the step shown (recorded 30.10.2026).
    let (coming, going) = (ease(LIGHT_RATE), ease(2.0 * LIGHT_RATE));
    let quiet = lights.bypass_change_detection();
    if let Some(key) = shown {
        quiet.lit.entry(key).or_insert(0.0);
    }
    let mut moved = false;
    quiet.lit.retain(|key, lit| {
        let target = if Some(*key) == shown { 1.0 } else { 0.0 };
        let factor = if target > *lit { coming } else { going };
        let next = *lit + (target - *lit) * factor;
        let next = if (next - target).abs() < 1e-3 {
            target
        } else {
            next
        };
        if (next - *lit).abs() > f32::EPSILON {
            moved = true;
            *lit = next;
        }
        target > 0.0 || *lit > 0.0
    });
    if moved {
        lights.set_changed();
    }
    let lit = |key| lights.lit.get(&key).copied().unwrap_or(0.0);
    for (glow, mut border, mut shadow) in &mut tiles {
        let on = lit((glow.player, glow.row));
        let want = BorderColor::all(crate::ambience::blend(glow.frame, palette::ACTIVE, on));
        if *border != want {
            *border = want;
        }
        let mut light = palette::ACTIVE.to_srgba();
        light.alpha = 0.55 * on;
        let colour = Color::from(light);
        let blur = Val::Px(NOW_GLOW * on);
        let spread = Val::Px(NOW_SPREAD * on);
        if let Some(first) = shadow.first()
            && (first.color != colour || first.blur_radius != blur || first.spread_radius != spread)
            && let Some(first) = shadow.first_mut()
        {
            first.color = colour;
            first.blur_radius = blur;
            first.spread_radius = spread;
        }
    }
    for (fill, mut ground) in &mut fills {
        let mut gold = palette::ACTIVE.to_srgba();
        gold.alpha *= lit(fill.0);
        let want = Color::from(gold);
        if ground.0 != want {
            ground.0 = want;
        }
    }
    // The glyph and the label are text, the stop accent a node. A text
    // node has a ground of its own too (every `Node` does), which must stay
    // clear: inked, the glyph and the label were solid blocks.
    for (ink, text, ground) in &mut inks {
        let want = crate::ambience::blend(ink.rest, palette::PARCHMENT_INK, lit(ink.key));
        match (text, ground) {
            (Some(mut text), _) => {
                if text.0 != want {
                    text.0 = want;
                }
            }
            (None, Some(mut ground)) => {
                if ground.0 != want {
                    ground.0 = want;
                }
            }
            (None, None) => {}
        }
    }
}

/// The combat line: where the next declaration points, and how many stand.
///
/// `None` when there is nothing to aim — a two-player game with no
/// planeswalkers has exactly one thing to attack, and a line saying so every
/// combat would be noise. The declaration count is still worth saying, so the
/// line survives that case whenever anything has been declared.
#[must_use]
pub(super) fn combat_line(
    interaction: &baylee_client_core::Interaction,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    texts: &crate::cardtext::CardTexts,
    lang: Lang,
) -> Option<String> {
    // `focus_position` answers for a target prompt as well now, and the aim
    // there points at the *first* half of the pair — a candidate, not a
    // defender — so `combat_focus` has nothing to name and this line would
    // read "aiming at nothing (1 of 3)" over an ordinary card choice. The one
    // caller filters on `is_combat` already; the guard is here so the
    // function's name is true whoever calls it.
    if !interaction.is_combat() {
        return None;
    }
    let (position, count) = interaction.focus_position()?;
    let declared = interaction.declared();
    let aiming = count > 1;
    if !aiming && declared == 0 {
        return None;
    }
    let aim = aiming.then(|| {
        let target = match interaction.combat_focus() {
            CombatFocus::Defender(Defender::Player(p)) => statics.map_or_else(
                || Phrase::ASeat.text(lang).to_string(),
                |s| s.seat_name(p).to_string(),
            ),
            CombatFocus::Defender(Defender::Planeswalker(o) | Defender::Battle(o))
            | CombatFocus::Attacker(o) => view.object(o).map_or_else(
                || Phrase::APermanent.text(lang).to_string(),
                |o| crate::face::name_of(o, view, texts),
            ),
            CombatFocus::None => Phrase::AimingAtNothing.text(lang).to_string(),
        };
        Phrase::AimedAt.fill(
            lang,
            &[&target, &(position + 1).to_string(), &count.to_string()],
        )
    });
    let standing =
        (declared > 0).then(|| Phrase::DeclaredCount.fill(lang, &[&declared.to_string()]));
    Some(
        [aim, standing]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("  ·  "),
    )
}

/// What is coming at each defender, and whether any of it reaches this seat.
///
/// The counterpart to [`combat_line`], and it answers a different question.
/// That line is about the declaration this seat is *making*; this one is
/// about the fight as it stands, so it is drawn whether or not this seat is
/// the one being asked — an attack made against you while you wait for the
/// blocker step is the thing you most need to read.
///
/// `true` in the second half means something unblocked is aimed at this seat,
/// which is the one case the line is worth drawing in a colour that carries.
#[must_use]
pub(super) fn incoming_line(
    view: &PlayerView,
    interaction: Option<&baylee_client_core::Interaction>,
    statics: Option<&GameStatic>,
    texts: &crate::cardtext::CardTexts,
    lang: Lang,
) -> Option<(String, bool)> {
    let combat = Combat::read(view, interaction);
    if combat.tallies.is_empty() {
        return None;
    }
    let name = |end: LineEnd| match end {
        LineEnd::Seat(p) if p == view.seat => Phrase::IncomingYou.text(lang).to_string(),
        LineEnd::Seat(p) => statics.map_or_else(
            || Phrase::ASeat.text(lang).to_string(),
            |s| s.seat_name(p).to_string(),
        ),
        LineEnd::Object(o) => view.object(o).map_or_else(
            || Phrase::APermanent.text(lang).to_string(),
            |o| crate::face::name_of(o, view, texts),
        ),
    };
    let threatened = combat
        .tallies
        .iter()
        .any(|t| t.unblocked > 0 && t.at == LineEnd::Seat(view.seat));
    let text = combat
        .tallies
        .iter()
        .map(|t| {
            Phrase::IncomingAt.fill(
                lang,
                &[
                    &name(t.at),
                    &t.attackers.to_string(),
                    &t.unblocked.to_string(),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join("  ·  ");
    Some((text, threatened))
}

/// Whether two seats play on the same side (same team when teams are
/// set; identical seats otherwise).
#[must_use]
pub fn same_team(statics: Option<&GameStatic>, a: PlayerId, b: PlayerId) -> bool {
    if a == b {
        return true;
    }
    let team_of = |p: PlayerId| {
        statics
            .and_then(|s| s.seats.iter().find(|i| i.player == p))
            .and_then(|i| i.team)
    };
    matches!((team_of(a), team_of(b)), (Some(ta), Some(tb)) if ta == tb)
}
