//! `cards/sorceries/mv_8/coalition_victory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::win::{EndReason, GameResult, Victor};

const P0: PlayerId = PlayerId::new(0);

fn coalition_victory() -> CardIndex {
    card_index("e9e25800-9ee7-40c9-b22d-611c7281c125")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn mahamoti_djinn() -> CardIndex {
    card_index("c39ea5f9-6ec0-4697-897b-779e326754a7")
}

/// Coalition Victory (e9e25800-9ee7-40c9-b22d-611c7281c125) — {3}{W}{U}{B}{R}{G}:
/// "You win the game if you control a land of each basic land type and a
/// creature of each color."
#[test]
fn coalition_victory_wins_only_when_you_control_all_five_land_types_and_colors() {
    let mut full = lk::game(
        SEED,
        &[
            plains(),
            island(),
            swamp(),
            mountain(),
            forest(),
            plains(),
            island(),
            swamp(),
            savannah_lions(),
            mahamoti_djinn(),
            lotleth_troll(),
            wild_colos(),
        ],
        &[],
        &[coalition_victory()],
        &[],
    );
    cast_from_hand(&mut full, P0, coalition_victory());
    pass_until(&mut full, |e| e.state().game_end.is_some());
    assert_eq!(
        full.state().game_end,
        Some(GameResult {
            winner: Some(Victor::Player(P0)),
            reason: EndReason::EffectWin,
        })
    );
}

/// Coalition Victory — requires *you* to control all five basic land types and
/// all five creature colors; opponent's permanents do not satisfy either half.
#[test]
fn coalition_victory_does_not_win_when_missing_a_land_type_or_creature_color() {
    // Missing Forest on P0's side (even though P1 controls Forest) -> does not win.
    let mut missing_land = lk::game(
        SEED,
        &[
            plains(),
            island(),
            swamp(),
            mountain(),
            chromatic_lantern(),
            savannah_lions(),
            mahamoti_djinn(),
            lotleth_troll(),
            wild_colos(),
        ],
        &[forest()],
        &[coalition_victory()],
        &[],
    );
    lk::float(
        &mut missing_land,
        P0,
        &[
            (ManaColor::White, 4),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    cast_with_floating(&mut missing_land, P0, coalition_victory());
    lk::settle(&mut missing_land);
    assert!(missing_land.state().game_end.is_none());

    // Missing Red creature on P0's side (even though P1 controls Wild Colos) -> does not win.
    let mut missing_color = lk::game(
        SEED,
        &[
            plains(),
            island(),
            swamp(),
            mountain(),
            forest(),
            savannah_lions(),
            mahamoti_djinn(),
            lotleth_troll(),
        ],
        &[wild_colos()],
        &[coalition_victory()],
        &[],
    );
    lk::float(&mut missing_color, P0, &[(ManaColor::White, 3)]);
    tap_all_mana(&mut missing_color, P0);
    cast_with_floating(&mut missing_color, P0, coalition_victory());
    lk::settle(&mut missing_color);
    assert!(missing_color.state().game_end.is_none());
}
