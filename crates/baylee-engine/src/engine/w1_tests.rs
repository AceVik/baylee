use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn hallowed_fountain() -> CardIndex {
    card_index("f1750962-a87c-49f6-b731-02ae971ac6ea")
}
fn glacial_fortress() -> CardIndex {
    card_index("027dd013-baa7-4111-b3c9-f4d1414e9c45")
}
fn indatha_triome() -> CardIndex {
    card_index("ec2b3779-55f7-4169-aa66-6312fb52721f")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset_with_hand(seed: u64, hand0: Vec<CardIndex>, bf0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0, bf0), mk(vec![], vec![])],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

fn play_land_from_hand(engine: &mut Engine<RegistryLookup>, p: PlayerId) {
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p && !legal.lands.is_empty() => {
                engine
                    .apply(
                        player,
                        PlayerAction::PlayLand {
                            card: legal.lands[0],
                        },
                    )
                    .unwrap();
                return;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 30, "no playable land for {p:?}");
    }
}

#[test]
fn shockland_pays_life_or_enters_tapped() {
    // Pay the 2 life.
    let mut engine = Engine::new(
        &preset_with_hand(41, vec![hallowed_fountain()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_before = engine.state().players[0].life;
    play_land_from_hand(&mut engine, p0);
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!("expected shockland choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[0].life, life_before - 2);
    let fountain = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        !engine
            .state()
            .object(fountain)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    // Decline → enters tapped.
    let mut engine = Engine::new(
        &preset_with_hand(42, vec![hallowed_fountain()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    play_land_from_hand(&mut engine, p0);
    let Pending::YesNo { .. } = engine.pending().clone() else {
        panic!("expected shockland choice")
    };
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let fountain = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        engine
            .state()
            .object(fountain)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn checkland_condition_is_evaluated() {
    // Without a Plains/Island: enters tapped.
    let mut engine = Engine::new(
        &preset_with_hand(43, vec![glacial_fortress()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    play_land_from_hand(&mut engine, p0);
    let fortress = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        engine
            .state()
            .object(fortress)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    // With a Plains: enters untapped.
    let mut engine = Engine::new(
        &preset_with_hand(44, vec![glacial_fortress()], vec![plains()]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    play_land_from_hand(&mut engine, p0);
    let fortress = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == glacial_fortress()))
        })
        .unwrap();
    assert!(
        !engine
            .state()
            .object(fortress)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn triome_cycling_from_hand_draws() {
    let mut engine = Engine::new(
        &preset_with_hand(
            45,
            vec![indatha_triome(), forest(), forest(), forest()],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    // Play 3 forests, tap, cycle the triome from hand. It was 2, and that is
    // not a detail of the harness: Indatha Triome cycles for `{3}`, its own
    // `//! Oracle:` header says so, and the card was built at `{2}`. This
    // test was written against the card rather than against the printing, so
    // it passed on the wrong number and pinned it there — which is how a
    // second reader (`xtask cross-read`) found the fault and this did not.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                // Play FORESTS only — the triome must stay in hand to cycle.
                if let Some(&land) = legal.lands.iter().find(|id| {
                    engine
                        .state()
                        .object(**id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
                }) {
                    engine
                        .apply(player, PlayerAction::PlayLand { card: land })
                        .unwrap();
                } else if !legal.mana_abilities.is_empty()
                    // Only where the mana can be spent. A pool empties as the
                    // step ends (CR 500.4), so a land tapped in the upkeep is
                    // a land wasted and this loop never reaches a cast.
                    && matches!(
                        engine.state().turn.phase,
                        Phase::FirstMain | Phase::SecondMain
                    )
                {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&(card, idx)) = legal.abilities.iter().find(|(id, _)| {
                    engine
                        .state()
                        .object(*id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index == indatha_triome()))
                }) {
                    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateAbility {
                                source: card,
                                ability_index: idx,
                            },
                        )
                        .unwrap();
                    // Triome is in the graveyard (DiscardSelf), ability on stack.
                    assert!(
                        engine
                            .state()
                            .zones
                            .list(ZoneLocation::Graveyard(p0))
                            .contains(&card)
                    );
                    // Resolve the draw.
                    let Pending::Priority { player, .. } = engine.pending().clone() else {
                        panic!()
                    };
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                    let Pending::Priority { player, .. } = engine.pending().clone() else {
                        panic!()
                    };
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                    assert_eq!(
                        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
                        hand_before - 1 + 1, // cycled one, drew one
                        "cycling should net one draw"
                    );
                    return;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        // 60 was enough for two land drops and is not enough for three.
        assert!(guard < 150, "triome never cycled");
    }
}

/// [`preset_with_hand`] with seat 0 on a chosen life total.
fn preset_at_life(seed: u64, hand0: Vec<CardIndex>, life: i32) -> GamePreset {
    let mut preset = preset_with_hand(seed, hand0, vec![]);
    preset.seats[0].starting_life = Some(life);
    preset
}

/// CR 119.4: a player may pay life while their life total is greater than or
/// **equal** to the payment, so a shockland asks a player on exactly two life
/// and takes their last two if they say yes.
///
/// The engine used to write that boundary as `life <= amount → cannot`, in
/// three places, which is one point of life too many: the land entered tapped
/// without a question being asked, and the player was never told there was
/// one. Paying is not the same act as losing: the loss is CR 104.3b/704.5a,
/// a separate rule that an effect saying a player cannot lose the game turns
/// off, so a payment which presupposed it would be refusing something
/// CR 119.4 permits. The margin that keeps the house AI
/// off this cliff is the AI's own (`activate::life_ok`, `life > amount + 5`),
/// which is where a policy belongs.
#[test]
fn a_shockland_asks_a_player_who_can_pay_their_last_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Engine::new(
        &preset_at_life(41, vec![hallowed_fountain()], 2),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    play_land_from_hand(&mut engine, p0);
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the shockland never asked a player who could pay; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount: 2 }
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[0].life, 0, "the two life were paid");
    // Whether the land came in untapped is `shockland_pays_life_or_enters_
    // tapped`'s question and cannot be asked here: the seat leaves the game
    // in the same `apply`, taking its permanents with it (CR 800.4a), so the
    // object is already gone by the time this returns.
    //
    // The other half of the same decision, and the reason it is one.
    assert!(
        matches!(engine.pending(), Pending::GameOver(_)),
        "a player at zero life loses to CR 704.5a; pending is {:?}",
        engine.pending()
    );
}

/// The boundary from the other side: one life is not two, so there is nothing
/// to ask about and the land enters tapped (CR 119.4, CR 614.1c).
#[test]
fn a_shockland_asks_nobody_who_cannot_pay_in_full() {
    let p0 = PlayerId::new(0);
    let mut engine = Engine::new(
        &preset_at_life(41, vec![hallowed_fountain()], 1),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    play_land_from_hand(&mut engine, p0);
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "a player on one life cannot pay two and must not be asked to"
    );
    assert_eq!(engine.state().players[0].life, 1, "nothing was paid");
    let fountain = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        engine
            .state()
            .object(fountain)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "an unpayable shockland enters tapped"
    );
}
