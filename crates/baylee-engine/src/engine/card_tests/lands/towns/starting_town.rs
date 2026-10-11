//! `cards/lands/towns/starting_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Starting Town: "This land enters tapped unless it's your first, second, or third turn of the game." / "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color."
/// It enters untapped during its controller's first three actual turns.
/// Activating ability 1 costs 1 life and produces one mana of any color chosen by the player.
#[test]
fn starting_town_enters_untapped_and_pays_life_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest()).hand(0, &[starting_town()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, starting_town());
    assert!(!entered_tapped(&engine, land));

    let before_life = engine.state().players[0].life;
    activate(&mut engine, p0, starting_town(), 1);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert_eq!(engine.state().players[0].life, before_life - 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn starting_town_counts_actual_own_turns_including_extra_turns() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[starting_town(); 4])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    for turn in 1..=4 {
        assert_eq!(e.state().players[0].turns_started, turn);
        let land = play_land(&mut e, p0, starting_town());
        assert_eq!(entered_tapped(&e, land), turn == 4);
        if turn < 4 {
            e.dev_state_mut(p0).unwrap().extra_turns.push_front(p0);
            pass_until(&mut e, |e| {
                e.state().players[0].turns_started == turn + 1
                    && e.state().turn.phase == Phase::FirstMain
            });
        }
    }
    assert_eq!(
        e.state().players[1].turns_started,
        0,
        "four own turns before the opponent's first"
    );
}

#[test]
fn starting_town_on_an_opponents_turn_enters_tapped_even_before_your_fourth() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[starting_town()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    let land = in_hand(&e, p0, starting_town()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            land,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority")
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(e.state().players[0].turns_started, 1);
    assert!(is_tapped(&e, land));
}
