use super::*;

/// Dimmest first. `Standing`'s own declaration order is not this one —
/// it is written in the order the *reading* code asks its questions —
/// so the ranking a mat draws is stated here rather than assumed from
/// the enum.
const RANKED: [Standing; 4] = [
    Standing::Lost,
    Standing::Waiting,
    Standing::Active,
    Standing::Asked,
];

fn mood(local: bool, standing: Standing) -> Mood {
    Mood {
        local,
        standing,
        // These tests are about brightness, which `on_turn` does not
        // touch: it drives the rim light and nothing else. A fixed
        // `false` keeps them measuring the one thing they measure.
        on_turn: false,
    }
}

/// The mat's tint is neutral, so this multiplies white and 1.0 is the
/// ceiling: two moods above it are one mood as far as a player can see.
///
/// The old scale ran to 1.311 and was safe only because it was applied to
/// an accent colour first. Moving the accent into the rim's texture — the
/// fix for a local mat that read as brass — is what made this a bound,
/// and it is asserted rather than remembered because the two changes are
/// in different files and nothing else connects them.
#[test]
fn no_mood_asks_for_more_light_than_white() {
    for local in [false, true] {
        for standing in RANKED {
            let value = zone_brightness(mood(local, standing));
            assert!(
                value > 0.0 && value <= 1.0,
                "a mat drawn at {value} is clipped, not bright"
            );
        }
    }
}

#[test]
fn a_dimmer_standing_is_always_drawn_dimmer() {
    for local in [false, true] {
        for pair in RANKED.windows(2) {
            let (dim, bright) = (
                zone_brightness(mood(local, pair[0])),
                zone_brightness(mood(local, pair[1])),
            );
            assert!(
                dim < bright,
                "{:?} and {:?} are drawn {dim} and {bright}",
                pair[0],
                pair[1]
            );
        }
    }
}

/// Whose mat is mine is answered by the gilt rim. Brightness answers who
/// everyone is waiting for, and the two must not compete: a local seat
/// idling has to stay dimmer than an opponent one rank above it, or the
/// felt points at the wrong player on every priority pass.
#[test]
fn a_standing_always_outranks_being_the_local_seat() {
    for pair in RANKED.windows(2) {
        let mine = zone_brightness(mood(true, pair[0]));
        let theirs = zone_brightness(mood(false, pair[1]));
        assert!(
            mine < theirs,
            "my {:?} mat at {mine} outshines their {:?} at {theirs}",
            pair[0],
            pair[1]
        );
    }
}
