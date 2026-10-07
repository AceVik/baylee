//! `cards/creatures/mv_2/plant_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plant Elemental is `{1}{G}` for a 3/4 whose whole text is one enters
/// trigger: "sacrifice it unless you sacrifice a Forest". Two boards play the
/// two branches of that "unless". With three Forests and an Elf standing, the
/// trigger asks which Forest is given up, the Elemental stays, and the menu it
/// offered can be read for what it leaves off — so the filter is a Forest and
/// not merely "a permanent you control". With no Forest anywhere — two
/// Llanowar Elves pay the `{1}{G}` — there is nothing to give up and the
/// trigger takes the Elemental itself, which is what says the fallback is a
/// real sacrifice rather than a clause that never fires.
#[test]
#[allow(clippy::too_many_lines)] // two boards, one printed sentence, one branch of it each
fn plant_elemental_is_sacrificed_unless_a_forest_is_given_up_for_it() {
    /// Answers whatever the enters-trigger asks: `pay` sacrifices the first
    /// thing the cost offers and declines everything otherwise. Returns the
    /// last menu the cost published, which is the only place the filter that
    /// drew it is legible.
    fn settle(engine: &mut Engine<RegistryLookup>, pay: bool) -> Vec<ObjectId> {
        let mut menu = Vec::new();
        for _ in 0..30 {
            match engine.pending().clone() {
                Pending::Priority { .. } if stack_is_empty(engine) => return menu,
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::YesNo { player, .. } => {
                    engine.apply(player, PlayerAction::YesNo(pay)).unwrap();
                }
                Pending::ChooseCards {
                    player,
                    options,
                    min,
                    ..
                } => {
                    if pay {
                        let first = options
                            .first()
                            .copied()
                            .expect("the cost names which Forest is given up");
                        menu = options;
                        engine
                            .apply(
                                player,
                                PlayerAction::ChooseObjects {
                                    objects: vec![first],
                                },
                            )
                            .unwrap();
                    } else {
                        assert_eq!(min, 0, "a may-cost is declined by choosing nothing");
                        engine
                            .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                            .unwrap();
                    }
                }
                other => panic!("unexpected while the enters-trigger resolves: {other:?}"),
            }
        }
        panic!("the enters-trigger never settled");
    }

    let p0 = PlayerId::new(0);

    // Feeding it a Forest. The Elf is the counter-half of the cost's filter:
    // a creature this seat controls, on the menu only if "a Forest" went
    // unread.
    let mut fed = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[plant_elemental()])
        .start();
    keep_mulligans(&mut fed);
    assert!(walk_to_own_main(&mut fed, p0), "p0 reaches its own main");
    let elf = on_battlefield(&fed, p0, llanowar_elves()).expect("the Elf is out");
    cast_from_hand(&mut fed, p0, plant_elemental());
    let menu = settle(&mut fed, true);

    assert_eq!(
        all_on_battlefield(&fed, p0, forest()).len(),
        2,
        "one Forest paid for the Elemental's life"
    );
    assert!(
        in_graveyard(&fed, p0, forest()).is_some(),
        "and a sacrificed land goes to its owner's graveyard"
    );
    assert!(
        !menu.contains(&elf),
        "\"a Forest\" is the whole of the cost — the Elf beside them is a \
         permanent this seat controls and no part of the menu: {menu:?}"
    );
    let elemental = on_battlefield(&fed, p0, plant_elemental())
        .expect("the Elemental is still standing, which is what it paid for");
    assert_eq!(pt(&fed, elemental), (3, 4), "the body the card prints");
    assert!(
        on_battlefield(&fed, p0, llanowar_elves()).is_some(),
        "and nothing else on the board moved"
    );

    // Nothing to give up, and the two Elves are the mana: the `{1}{G}` is
    // paid in full while not one Forest is anywhere on the table.
    let mut starved = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[plant_elemental()])
        .start();
    keep_mulligans(&mut starved);
    assert!(
        walk_to_own_main(&mut starved, p0),
        "p0 reaches its own main"
    );
    cast_from_hand(&mut starved, p0, plant_elemental());
    settle(&mut starved, false);

    assert!(
        on_battlefield(&starved, p0, plant_elemental()).is_none(),
        "\"sacrifice it unless you sacrifice a Forest\" — with no Forest to \
         give, the Elemental is the only permanent the clause can take"
    );
    assert!(
        in_graveyard(&starved, p0, plant_elemental()).is_some(),
        "and the sacrifice is a real one, so the card is in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&starved, p0, llanowar_elves()).len(),
        2,
        "the Elves were never part of the price, so neither of them paid it"
    );
}
