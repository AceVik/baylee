//! Revealed draw replacements and exact-card delayed/draw memory (CR 614.11b, 603.7c).
use super::*;

mod continuation;
use crate::event::Cause;
use crate::zone::{Zone, ZonePosition};
use baylee_cards_dsl::{Amount, Filter, Modifier, PlayerRel, ReplacementRule, Trigger};

const DRAW: u32 = 7980;
const REVEAL: u32 = 7981;
const DENY: u32 = 7982;
const EXILER: u32 = 7983;
const DREDGE: u32 = 7984;
const PLAIN: u32 = 7985;
const SKIPPER: u32 = 7986;
const LENG: u32 = 7987;
const LOCK: u32 = 7988;

static DRAW_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::DrawThenDiscardDrawn {
            draw: Amount::Fixed(2),
            discard: 1,
        }],
        None,
    ),
    free(
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        None,
    ),
    free(
        &[Effect::DiscardHand {
            who: PlayerRel::You,
        }],
        None,
    ),
];
static REVEAL_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Replacement(ReplacementRule::RevealDrawPutMatchingInGraveyard(
        &Filter::CREATURE,
    )),
    baylee_cards_dsl::static_ability!(Filter::Any, Modifier::RevealYourHand),
    free(&[Effect::SacrificeSelf], None),
];
static DENY_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
    ReplacementRule::RevealDrawOthersMayPayLife(2),
)];
static EXILER_ABILITIES: &[AbilityDef] = &[
    free(&[Effect::ExileTopFaceDownUntilYourNextEndStep], None),
    free(&[Effect::SacrificeSelf], None),
    baylee_cards_dsl::triggered!(
        Trigger::Discarded {
            who: PlayerRel::You,
            filter: &Filter::Any
        },
        &[Effect::ExileDiscardedCardFromYourGraveyard]
    ),
];
static DREDGE_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(ReplacementRule::Dredge(1))];
static SKIP_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::static_ability!(
        Filter::Any,
        Modifier::SkipDrawStep {
            who: PlayerRel::You
        }
    ),
    baylee_cards_dsl::triggered!(
        Trigger::StepBegin {
            step: baylee_cards_dsl::StepKind::Draw,
            whose: PlayerRel::You
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(7)
        }]
    ),
    free(&[Effect::SacrificeSelf], None),
];
static LENG_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
    ReplacementRule::MayDiscardToLibraryTop,
)];
static LOCK_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::Any,
    Modifier::LifeTotalCantChange {
        who: PlayerRel::You
    }
)];
fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            LENG,
            face("Discard destination probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            LENG_ABILITIES,
        ),
        card(
            LOCK,
            face("Life payment lock", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            LOCK_ABILITIES,
        ),
        card(
            SKIPPER,
            face("Draw step skip", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SKIP_ABILITIES,
        ),
        card(
            DRAW,
            face("Draw memory probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DRAW_ABILITIES,
        ),
        card(
            REVEAL,
            face("Reveal match probe", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            REVEAL_ABILITIES,
        ),
        card(
            DENY,
            face("Draw denial probe", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DENY_ABILITIES,
        ),
        card(
            EXILER,
            face("Delayed hidden probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            EXILER_ABILITIES,
        ),
        card(
            DREDGE,
            creature_face("Draw substitute", "{0}", 1, 1),
            KeywordSet::EMPTY,
            DREDGE_ABILITIES,
        ),
        card(
            PLAIN,
            face("Old hand card", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}
fn game(board: &[u32]) -> Bench {
    let mut e = bench(
        7980,
        cards(),
        [Seat::with(board).holding(&[PLAIN]), Seat::default()],
    );
    to_main(&mut e, me());
    e
}
fn start(e: &mut Bench, source: u32, index: u32) {
    let object = the(e, ZoneLocation::Battlefield, source);
    activate(e, me(), object, index);
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(them(), PlayerAction::PassPriority).unwrap();
}
fn top(e: &Bench) -> ObjectId {
    *e.state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap()
}
fn choose(e: &mut Bench, cards: Vec<ObjectId>) {
    e.apply(me(), PlayerAction::ChooseObjects { objects: cards })
        .unwrap();
}

#[test]
fn draw_memory_excludes_old_hand_and_replacement_returns() {
    let mut e = game(&[DRAW, DREDGE]);
    let dredge = the(&e, ZoneLocation::Battlefield, DREDGE);
    e.state
        .move_object(
            dredge,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    start(&mut e, DRAW, 0);
    choose(&mut e, vec![dredge]);
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        panic!("{:?}", e.pending())
    };
    assert_eq!((*min, *max), (1, 1));
    assert_eq!(options.len(), 1, "only the second, actual draw is eligible");
    assert!(!options.contains(&dredge));
    let drawn = options[0];
    choose(&mut e, vec![drawn]);
    settle(&mut e, me());
    assert_eq!(e.state().object(drawn).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(dredge).unwrap().zone, Zone::Hand);
    assert_eq!(objects(&e, ZoneLocation::Hand(me()), PLAIN).len(), 1);
}

#[test]
fn reveal_matching_draw_moves_creature_but_noncreature_draws_and_only_own_hand_is_public() {
    let mut e = game(&[DRAW, REVEAL, DREDGE]);
    assert!(e.state().hand_revealed(me()));
    assert!(!e.state().hand_revealed(them()));
    let creature = the(&e, ZoneLocation::Battlefield, DREDGE);
    e.state
        .move_object(
            creature,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    let before = e.state().per_turn.draws[0];
    start(&mut e, DRAW, 1);
    settle(&mut e, me());
    assert_eq!(e.state().object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().per_turn.draws[0], before);
    // Remove dredge from the graveyard to isolate the noncreature branch.
    e.state
        .move_object(
            creature,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    let land = top(&e);
    start(&mut e, DRAW, 1);
    settle(&mut e, me());
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().per_turn.draws[0], before + 1);
    start(&mut e, REVEAL, 2);
    settle(&mut e, me());
    assert!(!e.state().hand_revealed(me()));
}

#[test]
fn draw_denial_asks_other_player_and_replaces_without_discarding() {
    for pay in [false, true] {
        let mut e = game(&[DRAW, DENY]);
        let card = top(&e);
        let life = e.state().players[1].life;
        let before = e.state().per_turn.draws[0];
        start(&mut e, DRAW, 1);
        assert!(matches!(e.pending(), Pending::YesNo { player, .. } if *player == them()));
        assert!(e.apply(me(), PlayerAction::YesNo(pay)).is_err());
        e.apply(them(), PlayerAction::YesNo(pay)).unwrap();
        settle(&mut e, me());
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if pay { Zone::Graveyard } else { Zone::Hand }
        );
        assert_eq!(e.state().players[1].life, life - if pay { 2 } else { 0 });
        assert_eq!(e.state().per_turn.draws[0], before + u32::from(!pay));
        assert!(!e.journal().entries().iter().any(|entry| matches!(entry.event, crate::event::GameEvent::Discarded { object, .. } if object == card)));
    }
}

#[test]
fn delayed_hidden_card_returns_on_own_end_step_after_source_is_gone() {
    let mut e = game(&[EXILER]);
    let card = top(&e);
    start(&mut e, EXILER, 0);
    settle(&mut e, me());
    let object = e.state().object(card).unwrap();
    assert_eq!(object.zone, Zone::Exile);
    assert!(object.status.contains(crate::object::Status::FACE_DOWN));
    assert!(object.riders.iter().any(|r| matches!(r, crate::object::Rider::ExileVisibility { seen, follow_source: false } if seen.is_empty())));
    start(&mut e, EXILER, 1);
    settle(&mut e, me());
    walk_until(&mut e, |e| {
        e.state().turn.step == Step::End && e.state().zones.stack_is_empty()
    });
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Hand);
    assert!(
        !e.state()
            .object(card)
            .unwrap()
            .status
            .contains(crate::object::Status::FACE_DOWN)
    );
}

#[test]
fn delayed_hidden_return_does_not_follow_a_new_exile_incarnation() {
    let mut e = game(&[EXILER]);
    let card = top(&e);
    start(&mut e, EXILER, 0);
    settle(&mut e, me());
    e.state
        .move_object(
            card,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    e.state
        .move_object(
            card,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    walk_until(&mut e, |e| {
        e.state().turn.step == Step::End && e.state().zones.stack_is_empty()
    });
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
}

#[test]
fn discard_trigger_exiles_only_the_original_graveyard_successor() {
    for move_away in [false, true] {
        let mut e = game(&[DRAW, EXILER]);
        let card = the(&e, ZoneLocation::Hand(me()), PLAIN);
        start(&mut e, DRAW, 2);
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Graveyard);
        if move_away {
            e.state
                .move_object(
                    card,
                    ZoneLocation::Exile(me()),
                    ZonePosition::Top,
                    Cause::Setup,
                )
                .unwrap();
            e.state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(me()),
                    ZonePosition::Top,
                    Cause::Setup,
                )
                .unwrap();
        }
        settle(&mut e, me());
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if move_away {
                Zone::Graveyard
            } else {
                Zone::Exile
            }
        );
    }
}
