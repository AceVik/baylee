//! `cards/creatures/mv_3/furnace_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Furnace Spirit prints haste and "{R}: This creature gets +1/+0 until end of
/// turn", and this scenario plays both halves inside the one turn the first one
/// is about: the Spirit is cast, pumps itself twice off the red the five
/// Mountains left floating, and then attacks — which a 1/1 that arrived this
/// turn may only do because of the printed haste (CR 302.6). Reading the card
/// file cannot tell a real {R} from a label on a free ability, so the pool is
/// asserted around each activation, and the untapped Llanowar Elves beside it
/// stays a printed 1/1: the pump names `Filter::This` and never the board.
#[test]
fn furnace_spirit_attacks_the_turn_it_arrives_and_pumps_only_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(37, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[furnace_spirit()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Mountains are five red. The Elf is named as the printing kept back:
    // it is the bystander the pump has to leave alone, and its own `{T}` would
    // put a green in the pool that no count below accounts for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains tapped for five red"
    );

    cast_with_floating(&mut engine, p0, furnace_spirit());
    pass_until(&mut engine, stack_is_empty);
    let spirit = on_battlefield(&engine, p0, furnace_spirit()).expect("the Spirit resolved");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    assert_eq!(pt(&engine, spirit), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::HASTE),
        "the printed haste reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "three of the five paid {{2}}{{R}}, and nothing else moved"
    );

    // {R}: this creature gets +1/+0. No target question — the pump names
    // `Filter::This` — so the body and the pool are the whole of what there is
    // to read afterwards.
    activate(&mut engine, p0, furnace_spirit(), 0);
    assert!(!stack_is_empty(&engine), "a pump is no mana ability");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, spirit), (2, 1), "one activation is +1/+0");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and one red left the pool for it"
    );

    // A second activation: "until end of turn" stacks, and a cost that is
    // really paid is a cost the pool can run out of.
    activate(&mut engine, p0, furnace_spirit(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, spirit), (3, 1), "two activations, +2/+0");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the last red is spent"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the pump reaches the Spirit and no other creature"
    );

    // Haste is what lets a creature that entered this turn attack (CR 302.6),
    // and the three power it carries is what the pump added.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&spirit),
        "haste: the Spirit arrived this turn and is still offered as an \
         attacker: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(spirit, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers are
    // declared, so that predicate stops the walk *before* the combat damage
    // step and p1 would still read 20 (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "a 1/1 that pumped itself twice deals 3 combat damage"
    );
}
