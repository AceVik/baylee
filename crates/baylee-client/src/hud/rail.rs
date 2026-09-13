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
//! So did [`light_the_current_step`] and [`flash_the_designation`], and those
//! two are **waiting**: nothing spawns a [`PhaseNow`] or a [`Designation`]
//! between this commit and the one that gives the bar its baton and its
//! hinge-light, so both systems run over an empty query. They are kept rather
//! than rewritten because what they know is not the rail — it is that a light
//! eased from zero at spawn *is* the transition when the tree is rebuilt
//! whole, and that a flash anchored to an entity restarts every time the
//! pointer moves. Their tests spawn the markers by hand and hold both of
//! those.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::combat::{Combat, LineEnd};

/// Icon and the short rail label for a rail row.
pub(super) fn row_visual(row: RailRow) -> (char, &'static str) {
    match row {
        // A rotate-back arrow rather than the sun it used to be: the sun is
        // the day designation's glyph now, and two of them a hundred pixels
        // apart in one strip would say the untap step *is* the daytime.
        // Untapping is turning a card back, which is what this draws.
        RailRow::Untap => ('\u{f0e2}', "UNT"),
        RailRow::Upkeep => ('\u{f0ad}', "UPK"),
        RailRow::Draw => ('\u{f063}', "DRW"),
        RailRow::Main1 => ('\u{f024}', "M1"),
        RailRow::CombatBegin => ('\u{f71d}', "CBT"),
        RailRow::Attackers => ('\u{f70c}', "ATK"),
        RailRow::Blockers => ('\u{f3ed}', "BLK"),
        RailRow::Damage => ('\u{f6e2}', "DMG"),
        RailRow::CombatEnd => ('\u{f11e}', "EOC"),
        RailRow::Main2 => ('\u{f024}', "M2"),
        RailRow::EndStep => ('\u{f253}', "END"),
        RailRow::Cleanup => ('\u{f51a}', "CLN"),
    }
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
/// It writes the border and the shadow and leaves the background alone, for
/// [`light_the_current_step`]'s reason: two systems writing one component is
/// a fight the frame order decides. The decay is anchored to the change
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
        let flash = if still {
            0.0
        } else {
            (-FLASH_DECAY * (now - state.at)).exp()
        };
        let tone = match block.0 {
            baylee_view::DayNight::Day => palette::PARCHMENT,
            baylee_view::DayNight::Night => palette::INK,
        };
        let mut edge = tone.to_srgba();
        edge.alpha = 0.55 * flash;
        *border = BorderColor::all(Color::from(edge));
        if let Some(first) = shadow.first_mut() {
            let mut glow = tone.to_srgba();
            glow.alpha = 0.55 * flash;
            first.color = Color::from(glow);
            first.blur_radius = Val::Px(NOW_GLOW * flash);
            first.spread_radius = Val::Px(NOW_SPREAD * flash);
        }
    }
}

/// How fast the current step's button lights up, per second.
///
/// The same exponential the camera and the lobby's buttons use, at a rate
/// that puts the light at about nine tenths after a quarter of a second: long
/// enough to read as a movement, short enough that a player pressing through
/// four steps in a row is never waiting for the rail to catch up.
const LIGHT_RATE: f32 = 9.0;

/// How far the light around the current step's button reaches, and how far it
/// stands off the button's own edge.
const NOW_GLOW: f32 = 10.0;
const NOW_SPREAD: f32 = 2.0;

/// Lights the step the game is in.
///
/// A system rather than a colour written at build time, because the HUD tree
/// is rebuilt whole whenever anything in [`HudRevision`] changes — a step
/// change is one of those, so the new button is *born* current and the old
/// one no longer exists. Easing from zero at spawn is therefore exactly the
/// transition: the light arrives on the new step over a quarter of a second
/// instead of cutting there.
///
/// It writes the border and the shadow and leaves the background alone,
/// because [`Feel`] owns that one and two systems writing the same component
/// is a fight the frame order decides.
pub fn light_the_current_step(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut steps: Query<(&mut PhaseNow, &mut BorderColor, &mut BoxShadow)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let step = if still {
        1.0
    } else {
        1.0 - (-LIGHT_RATE * time.delta_secs()).exp()
    };
    for (mut now, mut border, mut shadow) in &mut steps {
        if now.lit >= 1.0 {
            continue;
        }
        now.lit = (now.lit + (1.0 - now.lit) * step).min(1.0);
        if now.lit > 0.999 {
            now.lit = 1.0;
        }
        let mut tone = palette::ACTIVE.to_srgba();
        tone.alpha = now.lit;
        *border = BorderColor::all(Color::from(tone));
        if let Some(first) = shadow.first_mut() {
            let mut glow = palette::ACTIVE.to_srgba();
            glow.alpha = 0.55 * now.lit;
            first.color = Color::from(glow);
            first.blur_radius = Val::Px(NOW_GLOW * now.lit);
            first.spread_radius = Val::Px(NOW_SPREAD * now.lit);
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
            CombatFocus::Defender(Defender::Planeswalker(o)) | CombatFocus::Attacker(o) => {
                view.object(o).map_or_else(
                    || Phrase::APermanent.text(lang).to_string(),
                    |o| crate::face::name_of(o, view, texts),
                )
            }
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
