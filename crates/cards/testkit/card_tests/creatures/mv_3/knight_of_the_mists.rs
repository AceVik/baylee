//! `cards/creatures/mv_3/knight_of_the_mists.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn knight_of_the_mists() -> CardIndex {
    card_index("e9893534-3d05-42c2-93ff-f28ffae89aa3")
}

fn knight_of_valor() -> CardIndex {
    card_index("f2edb543-de33-4eed-892c-98038eec96e2")
}

/// P0 with four Islands and the Knight in hand, P1 with a Knight (of Valor) and a Grizzly Bears.
fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[knight_of_valor(), grizzly_bears()])
        .hand(0, &[knight_of_the_mists()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Casts the Knight off three Islands, aims its trigger at `target` and stops at the {U} question.
/// Returns the target menu and the fourth Island, the one that is left to pay with.
#[track_caller]
fn cast_and_aim(
    engine: &mut Engine<RegistryLookup>,
    target: ObjectId,
) -> (Vec<ObjectId>, ObjectId) {
    let islands = lands_of(engine, P0);
    let spell_lands: Vec<ObjectId> = islands.iter().copied().take(3).collect();
    let price_land = islands[3];
    tap_mana_where(engine, P0, |land| spell_lands.contains(&land));
    cast_with_floating(engine, P0, knight_of_the_mists());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let menu = aim_at(engine, P0, target);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the Knight's controller is asked");
    assert_eq!(cost, baylee_core::mana!("{U}"), "the price is {{U}}");
    (menu, price_land)
}

/// Knight of the Mists ({2}{U}, 2/2 Human Knight): "When this creature enters, you may pay {U}.
/// If you don't, destroy target Knight and it can't be regenerated." Paying {U} spares the
/// target; the menu is every Knight (the Knight itself too), not the Grizzly Bears.
#[test]
fn knight_of_the_mists_paying_u_spares_the_target_knight() {
    let mut engine = board();
    let enemy_knight = on_battlefield(&engine, P1, knight_of_valor()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    let (menu, price_land) = cast_and_aim(&mut engine, enemy_knight);
    let mists = on_battlefield(&engine, P0, knight_of_the_mists()).unwrap();
    assert!(
        menu.contains(&enemy_knight),
        "the opponent's Knight is a target"
    );
    assert!(
        menu.contains(&mists),
        "so is the Knight of the Mists itself"
    );
    assert!(
        !menu.contains(&bears),
        "a creature that is not a Knight is not"
    );

    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, P0, |land| land == price_land);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, price_land), "the {{U}} was paid");
    assert!(
        on_battlefield(&engine, P1, knight_of_valor()).is_some(),
        "paid: the Knight lives"
    );
    assert!(on_battlefield(&engine, P0, knight_of_the_mists()).is_some());
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
}

/// Declining the {U} destroys the target Knight and nothing else.
#[test]
fn knight_of_the_mists_not_paying_destroys_only_the_target_knight() {
    let mut engine = board();
    let enemy_knight = on_battlefield(&engine, P1, knight_of_valor()).unwrap();

    let (_, price_land) = cast_and_aim(&mut engine, enemy_knight);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, knight_of_valor()).is_some(),
        "not paid: the Knight is destroyed"
    );
    assert!(
        on_battlefield(&engine, P0, knight_of_the_mists()).is_some(),
        "the Knight of the Mists is not the target"
    );
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
    assert!(!is_tapped(&engine, price_land), "nothing was paid");
}
