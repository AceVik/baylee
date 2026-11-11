//! Granted abilities that sacrifice or deal damage as their exact grantor.
use super::*;
use baylee_cards_dsl::Filter;

const WEARER: u32 = 99_700;
const FIRST: u32 = 99_701;
const SECOND: u32 = 99_702;
const VICTIM: u32 = 99_703;

static GRANTOR_PING: AbilityDef = baylee_cards_dsl::activated!(
    baylee_cards_dsl::cost!(SacrificeGrantor),
    &[Effect::GrantorDealsDamage {
        amount: baylee_cards_dsl::Amount::Fixed(1),
        target: baylee_cards_dsl::TargetSpec::Player(baylee_cards_dsl::PlayerRel::EachOpponent),
    }]
);

static GRANTOR_DAMAGE: AbilityDef = baylee_cards_dsl::activated!(
    baylee_cards_dsl::cost!(SacrificeGrantor),
    &[Effect::GrantorDealsDamage {
        amount: baylee_cards_dsl::Amount::Fixed(1),
        target: baylee_cards_dsl::TargetSpec::Object(&Filter::CREATURE),
    }],
    targets = Some(TargetReq::one(baylee_cards_dsl::TargetSpec::Object(
        &Filter::CREATURE
    ))),
);

static ORDINARY_PING: AbilityDef = baylee_cards_dsl::activated!(
    Cost::TAP,
    &[Effect::DealDamage {
        amount: baylee_cards_dsl::Amount::Fixed(1),
        target: baylee_cards_dsl::TargetSpec::Player(baylee_cards_dsl::PlayerRel::EachOpponent),
    }]
);

static GRANT_PING: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::YOUR_CREATURE,
    baylee_cards_dsl::Modifier::GrantActivatedDefinition(&GRANTOR_PING)
)];
static GRANT_DAMAGE: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::YOUR_CREATURE,
    baylee_cards_dsl::Modifier::GrantActivatedDefinition(&GRANTOR_DAMAGE)
)];
static GRANT_ORDINARY: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::YOUR_CREATURE,
    baylee_cards_dsl::Modifier::GrantActivatedDefinition(&ORDINARY_PING)
)];

fn artifact(
    index: u32,
    abilities: &'static [AbilityDef],
    keywords: KeywordSet,
) -> &'static CardDef {
    card(
        index,
        face("Grantor probe", "{0}", TypeSet::ARTIFACT),
        keywords,
        abilities,
    )
}

fn bearer() -> &'static CardDef {
    card(
        WEARER,
        creature_face("Grant recipient probe", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    )
}

fn game(cards: Vec<&'static CardDef>, board: &[u32]) -> Bench {
    let mut engine = bench(99_700, cards, [Seat::with(board), Seat::default()]);
    to_main(&mut engine, me());
    engine
}

fn granted(engine: &Bench, wearer: ObjectId) -> Vec<u32> {
    offer(engine)
        .1
        .abilities
        .into_iter()
        .filter(|(source, index)| {
            *source == wearer && crate::choice::granted_slot(*index).is_some()
        })
        .map(|(_, index)| index)
        .collect()
}

#[test]
fn two_grantors_pay_with_their_own_incarnations_and_leave_the_recipient() {
    let mut engine = game(
        vec![
            bearer(),
            artifact(FIRST, GRANT_PING, KeywordSet::EMPTY),
            artifact(SECOND, GRANT_PING, KeywordSet::EMPTY),
        ],
        &[WEARER, FIRST, SECOND],
    );
    let wearer = the(&engine, ZoneLocation::Battlefield, WEARER);
    let first = the(&engine, ZoneLocation::Battlefield, FIRST);
    let second = the(&engine, ZoneLocation::Battlefield, SECOND);
    let abilities = granted(&engine, wearer);
    assert_eq!(abilities.len(), 2, "one ability per granting permanent");

    activate(&mut engine, me(), wearer, abilities[0]);
    let sacrificed = if engine.state.object(first).unwrap().zone == Zone::Graveyard {
        first
    } else {
        assert_eq!(engine.state.object(second).unwrap().zone, Zone::Graveyard);
        second
    };
    let survivor = if sacrificed == first { second } else { first };
    assert_eq!(engine.state.object(wearer).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        engine.state.object(survivor).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(granted(&engine, wearer).len(), 1, "the other grant remains");

    let ability = granted(&engine, wearer)[0];
    activate(&mut engine, me(), wearer, ability);
    assert_eq!(engine.state.object(survivor).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state.object(wearer).unwrap().zone, Zone::Battlefield);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        18,
        "each Equipment dealt its own one"
    );
}

#[test]
fn grantor_sacrifice_is_not_offered_after_loss_of_control_or_a_zone_change() {
    for case in 0..2 {
        let mut engine = game(
            vec![bearer(), artifact(FIRST, GRANT_PING, KeywordSet::EMPTY)],
            &[WEARER, FIRST],
        );
        let wearer = the(&engine, ZoneLocation::Battlefield, WEARER);
        let grantor = the(&engine, ZoneLocation::Battlefield, FIRST);
        assert_eq!(granted(&engine, wearer).len(), 1);
        if case == 0 {
            engine
                .state
                .object_mut(grantor)
                .unwrap()
                .set_controller(them());
        } else {
            engine
                .state
                .move_object(
                    grantor,
                    ZoneLocation::Graveyard(me()),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        engine.state.refresh_characteristics();
        engine.refresh_offer();
        assert!(
            granted(&engine, wearer).is_empty(),
            "case {case}: the ability cannot be activated without its exact payable grantor"
        );
    }
}

#[test]
fn sacrificed_grantors_deathtouch_and_lifelink_survive_a_new_incarnation() {
    let victim_card = card(
        VICTIM,
        creature_face("One-toughness victim", "{0}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let mut engine = bench(
        99_701,
        vec![
            bearer(),
            artifact(
                FIRST,
                GRANT_DAMAGE,
                KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK),
            ),
            victim_card,
        ],
        [Seat::with(&[WEARER, FIRST]), Seat::with(&[VICTIM])],
    );
    to_main(&mut engine, me());
    let wearer = the(&engine, ZoneLocation::Battlefield, WEARER);
    let grantor = the(&engine, ZoneLocation::Battlefield, FIRST);
    let victim = the(&engine, ZoneLocation::Battlefield, VICTIM);
    let initial = life(&engine, me());
    let ability = granted(&engine, wearer)[0];
    activate(&mut engine, me(), wearer, ability);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    assert_eq!(engine.state.object(grantor).unwrap().zone, Zone::Graveyard);

    engine
        .state
        .move_object(
            grantor,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .object_mut(grantor)
        .unwrap()
        .base_mut()
        .keywords = KeywordSet::EMPTY;
    engine.state.refresh_characteristics();
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(victim).unwrap().zone,
        Zone::Graveyard,
        "the sacrificed incarnation's deathtouch, not the returned object's keywords"
    );
    assert_eq!(
        life(&engine, me()),
        initial + 1,
        "and its lifelink gained one"
    );
}

#[test]
fn an_ordinary_granted_ping_still_taps_the_recipient_and_keeps_its_grantor() {
    let mut engine = game(
        vec![bearer(), artifact(FIRST, GRANT_ORDINARY, KeywordSet::EMPTY)],
        &[WEARER, FIRST],
    );
    let wearer = the(&engine, ZoneLocation::Battlefield, WEARER);
    let grantor = the(&engine, ZoneLocation::Battlefield, FIRST);
    let ability = granted(&engine, wearer)[0];
    activate(&mut engine, me(), wearer, ability);
    assert!(
        engine
            .state
            .object(wearer)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(
        engine.state.object(grantor).unwrap().zone,
        Zone::Battlefield
    );
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 19);
}
