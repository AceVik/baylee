//! An ability with `{X}` in its mana cost is offered before its mana is made
//! only while the mana its controller could make reaches its cost at X = 0
//! (`activation_reach`; CR 602.2b, 601.2g).

use super::activation_reach::Reach;
use super::testkit::*;
use super::*;
use baylee_core::generated::index;
use baylee_core::ids::{CardIndex, Defender};
use baylee_core::mana::{ManaColor, ManaCost};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// P0 holds Ballista Squad (`{X}{W}, {T}`: X damage to target attacking or
/// blocking creature) beside `lands` with nothing floating, attacks P1 with
/// Grizzly Bears, and is given priority in the declare attackers step, where
/// the Bears are a target. The Squad and that offer.
fn squad_beside(lands: &[CardIndex]) -> (ObjectId, LegalActions) {
    let mut mine = vec![index::BALLISTA_SQUAD, index::GRIZZLY_BEARS];
    mine.extend_from_slice(lands);
    let mut engine = Duel::new(8201, index::ISLAND).battlefield(0, &mine).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let squad = on_battlefield(&engine, P0, index::BALLISTA_SQUAD).expect("the Squad");
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("the Bears");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .expect("the Bears attack");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("priority after the attack: {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert!(engine.state().combat.is_attacking(bears));
    (squad, *legal)
}

/// Carbon b8, segment 3, attempt 42, seat 1, q126: Ballista Squad was
/// offered with an Island its only untapped land. At X = 0 its cost is
/// `{W}`, which no mana that seat could make pays, so taking it opened a
/// window that could only be reversed (CR 734.1). It is not offered; beside
/// a Plains it is, and its X is announced before the mana is made, as
/// before (CR 602.2b).
#[test]
fn an_x_ability_is_offered_only_where_the_mana_reaches_its_least_cost() {
    let (squad, legal) = squad_beside(&[index::ISLAND]);
    assert!(
        !legal.abilities.contains(&(squad, 0)),
        "no {{W}} with an Island alone: {:?}",
        legal.abilities
    );
    let (squad, legal) = squad_beside(&[index::ISLAND, index::PLAINS]);
    assert!(
        legal.abilities.contains(&(squad, 0)),
        "a Plains makes the {{W}}: {:?}",
        legal.abilities
    );
    assert!(
        !legal
            .unpaid_abilities
            .iter()
            .any(|(s, i, _)| (*s, *i) == (squad, 0)),
        "offered, and no hint beside it"
    );
}

fn reach(sources: &[(&[ManaColor], Option<u32>)]) -> Reach {
    let mut reach = Reach::default();
    for &(colors, amount) in sources {
        reach.add(colors, amount);
    }
    reach
}

/// The reading is loose and never refuses what the mana could pay: a source
/// counts toward each colour it makes and once in the total, a source whose
/// amount is unread opens its colours, an unread source makes anything,
/// a hybrid asks either colour and a Phyrexian symbol is paid with life
/// (CR 107.4f).
#[test]
fn a_reach_refuses_only_what_no_assignment_of_its_mana_pays() {
    use ManaColor::{Blue, Green, Red, White};
    let cost = ManaCost::parse;
    let island = reach(&[(&[Blue], Some(1))]);
    assert!(!island.pays(&cost("{W}")));
    assert!(island.pays(&cost("{U}")));
    assert!(!island.pays(&cost("{1}{U}")), "one mana in all");
    assert!(island.pays(&cost("{W/U}")));
    assert!(island.pays(&cost("{W/P}")), "life pays it");
    assert!(island.pays(&ManaCost::ZERO));

    let tundra = reach(&[(&[White, Blue], Some(1))]);
    assert!(tundra.pays(&cost("{W}")) && tundra.pays(&cost("{U}")));
    assert!(!tundra.pays(&cost("{W}{U}")), "one land, one mana");

    let pours = reach(&[(&[Green], None)]);
    assert!(pours.pays(&cost("{G}{G}{G}{4}")));
    assert!(!pours.pays(&cost("{R}")));

    let mut anything = reach(&[(&[Red], Some(1))]);
    anything.anything();
    assert!(anything.pays(&cost("{W}{W}{U}{5}")));

    assert!(
        !reach(&[(&[], Some(3))]).pays(&cost("{1}")),
        "a source of no colour makes nothing (CR 106.7)"
    );
}
