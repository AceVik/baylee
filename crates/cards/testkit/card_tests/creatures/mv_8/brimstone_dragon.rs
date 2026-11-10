//! `cards/creatures/mv_8/brimstone_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brimstone Dragon — `{6}{R}{R}`, a 6/6 Dragon with flying and haste.
///
/// One board reads both printed keywords, because each needs a different
/// witness. Haste is why the Dragon may be declared as an attacker in the
/// very combat step of the turn it landed (CR 702.10, CR 302.6): a Llanowar
/// Elves cast out of the same pool stands beside it, entered the same turn
/// and is declined for exactly that reason, so the offer is the rule and not
/// every creature on the battlefield. Flying is read from the other end — the
/// block the defending seat is offered names the Angel and not the grounded
/// Elf — which is the comparison that tells "creatures with flying" from
/// "creatures". Eight Mountains and one Forest are the whole price of both
/// spells, so the pool reads empty once they have resolved.
#[test]
fn brimstone_dragon_lands_as_a_six_six_with_flying_and_haste() {
    fn brimstone_dragon() -> CardIndex {
        card_index("b20302d9-eb45-4895-9597-cf3af1abce56")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board: Vec<CardIndex> = vec![mountain(); 8];
    board.push(forest());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[llanowar_elves(), serra_angel()])
        .hand(0, &[brimstone_dragon(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf first, out of the same nine mana the Dragon is about to spend:
    // it has to be *cast* this turn for the no-haste control below to mean
    // anything, and {{G}} is the Forest's while the eight Mountains are red.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, brimstone_dragon());
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, brimstone_dragon()).expect("the Dragon resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "eight Mountains and one Forest paid {{6}}{{R}}{{R}} and {{G}} to the \
         last mana, so nothing floating can stand in for either"
    );
    assert_eq!(pt(&engine, dragon), (6, 6), "the body the card prints");
    let granted = keywords(&engine, dragon);
    assert!(granted.contains(KeywordSet::FLYING), "flying");
    assert!(granted.contains(KeywordSet::HASTE), "haste");

    // The combat step of the turn the Dragon entered. A creature that came
    // under its controller's control this turn may not attack (CR 302.6)
    // unless something grants it haste, so both entries of this one offer are
    // the whole claim.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the turn belongs to the seat that cast it");
    assert!(
        attackers.contains(&dragon),
        "\"haste\": the Dragon is offered in the combat step of the turn it \
         entered, which summoning sickness alone would withhold: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elf),
        "and the Elf cast in the same main phase is not, so the list is the \
         rule and not every creature on the board: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(p1))],
            },
        )
        .expect("the Dragon came out of the list that offered it");

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the Angel is out");

    let mut can_block: Vec<ObjectId> = Vec::new();
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                assert_eq!(player, p1, "the defending seat names the blockers");
                can_block = blockers
                    .into_iter()
                    .filter(|option| option.attackers.contains(&dragon))
                    .map(|option| option.blocker)
                    .collect();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected between the attack and the block: {other:?}"),
        }
    }

    assert!(
        can_block.contains(&angel),
        "\"flying\" does not keep a blocker that also has it out: the Angel is \
         offered for the Dragon: {can_block:?}"
    );
    assert!(
        !can_block.contains(&their_elf),
        "and a creature without flying or reach may not block a flier \
         (CR 702.9b, CR 509.1b), so the pairing is the filter and not the \
         defender's whole board: {can_block:?}"
    );
    assert_eq!(
        can_block.len(),
        1,
        "the Angel is the whole of what may block it: {can_block:?}"
    );
}
