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
        // These tests are about brightness, which neither of these
        // touches: one drives the rim light and the other breaks it into
        // dashes, and both leave `zone_brightness` alone. A fixed `false`
        // keeps them measuring the one thing they measure.
        on_turn: false,
        held: false,
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

/// A chair handed to the house mid-game reaches the mat it is drawn on.
///
/// The path is roster → [`SeatPod::role`] → [`Mood`] → [`MatParams`], and the
/// middle step is the one that had to be got right. `sync_zones` skips a seat
/// whose `mood` and `accent` are both unchanged, so a flag carried *beside*
/// the mood would be written when the mat was first built and never again:
/// the chair would go to the house and keep a solid rim until something else
/// about that seat happened to move. Two moods that differ is what opens the
/// gate, and it is what this asserts — deleting `held` from `Mood` and
/// passing it to `mat_params` separately compiles, draws correctly on the
/// first frame, and fails here.
///
/// What it does *not* reach is `sync_zones` itself, which has no harness: it
/// wants two asset stores, a mesh and a `SceneIndex`. The predicate the gate
/// compares is what is held here, and the gate's own line is a read.
#[test]
fn a_chair_handed_to_the_house_changes_the_mat_it_is_drawn_on() {
    use baylee_client_core::board::SeatRole;

    let pod = |role: SeatRole| baylee_client_core::board::SeatPod {
        player: PlayerId::new(0),
        life: 20,
        poison: 0,
        energy: 0,
        hand_count: 0,
        library_count: 40,
        graveyard_count: 0,
        has_lost: false,
        is_local: true,
        is_active: true,
        is_awaited: true,
        role,
        lanes: Vec::new(),
        piles: Vec::new(),
        tokens: Vec::new(),
        threat: baylee_client_core::ThreatSummary::default(),
    };
    let (present, held) = (
        Mood::of(&pod(SeatRole::Present)),
        Mood::of(&pod(SeatRole::Away)),
    );
    assert!(
        held.held && !present.held,
        "the mood does not carry the chair"
    );
    assert!(
        present != held,
        "a chair going to the house leaves the mat's rebuild gate shut"
    );

    // And the house's own chair is not a held one. An AI seat was always an
    // AI seat; this mark is for a player's chair being covered until they
    // come back, and spending it on both would make it say nothing.
    assert_eq!(
        Mood::of(&pod(SeatRole::House)),
        present,
        "an AI chair is drawn as a held one"
    );

    // The flag reaches the shader as a number.
    let size = bevy::prelude::Vec2::new(12.0, 5.0);
    let of = |mood| {
        mat_params(
            bevy::prelude::Color::WHITE,
            size,
            mood,
            false,
            baylee_client_core::layout::LEDGE_IS_OUTER,
        )
    };
    let (on, off) = (of(held).held, of(present).held);
    assert!(
        (on - 1.0).abs() < f32::EPSILON && off.abs() < f32::EPSILON,
        "the shader is handed {on} for a held chair and {off} for a seated one"
    );
}
