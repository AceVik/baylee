//! `cards/creatures/mv_2/mistfolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mistfolk() -> CardIndex {
    card_index("d57409dc-22fa-40c1-a043-cab94d5f3b5f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Mistfolk (d57409dc-22fa-40c1-a043-cab94d5f3b5f) — {U}{U} 1/2 Creature — Illusion:
/// "{U}: Counter target spell that targets this creature."
#[test]
fn mistfolk_counters_a_spell_targeting_it_and_cannot_target_a_spell_aimed_elsewhere() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[mistfolk(), grizzly_bears(), island()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[unsummon(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mist = on_battlefield(&engine, P0, mistfolk()).expect("Mistfolk seated");
    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("Bears seated");

    // Pass priority to P1 so P1 can cast the first Unsummon targeting Grizzly Bears (not Mistfolk).
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, unsummon());
    aim_at(&mut engine, P1, bears);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    // P0 holds priority while Unsummon targeting Grizzly Bears is on the stack.
    assert!(
        !ability_offered(&engine, mist, 0),
        "Mistfolk's {{U}} ability is not offered when the spell on the stack targets another creature"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none(),
        "Unsummon on Grizzly Bears resolved"
    );

    // Now P1 casts the second Unsummon targeting Mistfolk.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, unsummon());
    aim_at(&mut engine, P1, mist);
    let spell_on_stack = on_stack(&engine, unsummon()).expect("Unsummon on stack");
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    assert!(
        ability_offered(&engine, mist, 0),
        "Mistfolk's {{U}} ability is offered when a spell on the stack targets Mistfolk"
    );
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, mistfolk(), 0);
    aim_at(&mut engine, P0, spell_on_stack);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, mistfolk()).is_some(),
        "Mistfolk countered the Unsummon targeting it and stays on the battlefield"
    );
}
