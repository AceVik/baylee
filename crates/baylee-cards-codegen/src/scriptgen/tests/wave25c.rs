//! Wave 25c unit tests: Wave 13b (`StepKind::SecondMain`,
//! `Trigger::AttackersDeclaredAtTarget`, LKI `ReplacementRule::ExileInsteadOfGraveyard`)
//! and Wave 13c (`Modifier::AdjustSpellCostByCast` with `SpellCastMetric`), plus
//! strict Paper-MTG / soundness refusals.

use super::*;

fn w25c_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Avatar".into(),
            "Beast".into(),
            "Cat".into(),
            "Citizen".into(),
            "Coward".into(),
            "Demon".into(),
            "Dragon".into(),
            "Druid".into(),
            "Dryad".into(),
            "Elder".into(),
            "Elf".into(),
            "Giant".into(),
            "Goblin".into(),
            "Golem".into(),
            "Halfling".into(),
            "Human".into(),
            "Kirin".into(),
            "Lizard".into(),
            "Merfolk".into(),
            "Minotaur".into(),
            "Mutant".into(),
            "Pirate".into(),
            "Plant".into(),
            "Samurai".into(),
            "Shaman".into(),
            "Soldier".into(),
            "Spirit".into(),
            "Survivor".into(),
            "Treefolk".into(),
            "Tyranid".into(),
            "Villain".into(),
            "Warlock".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec!["Clue".into(), "Food".into(), "Treasure".into()],
        land: vec![
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w25c(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &w25c_cats(), None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &w25c_cats(), None)
        )
    })
}

fn refused_w25c(text: &str) -> bool {
    transcode(&parse(text), &w25c_cats(), None).is_none()
}

// ---------------------------------------------------------------------------
// 1. StepKind::SecondMain (CR 505.1a)
// ---------------------------------------------------------------------------

#[test]
fn ordinal_and_postcombat_main_phase_triggers_and_phase_conditions() {
    // Neheb, the Eternal shape: `Phase$ Main2`.
    let neheb = read_w25c(
        "Name:Neheb Proxy\nManaCost:3 R R\nTypes:Legendary Creature Zombie Minotaur Warrior\nPT:4/6\n\
         T:Mode$ Phase | Phase$ Main2 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigMana\n\
         SVar:TrigMana:DB$ Mana | Produced$ R | Amount$ X\n\
         SVar:X:Count$LifeOppsLostThisTurn",
    );
    assert_eq!(
        neheb.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::PostcombatMain, whose: PlayerRel::You }, \
             &[Effect::mana_dynamic(ManaColor::Red, Amount::OpponentsLifeLostThisTurn)])"
        ]
    );

    // Survival / Kona shape: `Phase$ Main | PhaseCount$ 2` + tapped self condition.
    let kona = read_w25c(
        "Name:Kona Proxy\nManaCost:3 G\nTypes:Legendary Creature Beast Survivor\nPT:4/3\n\
         T:Mode$ Phase | Phase$ Main | PhaseCount$ 2 | ValidPlayer$ You | PresentDefined$ Self | \
         IsPresent$ Card.tapped | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert!(
        kona.abilities[0].contains(
            "triggered!(Trigger::StepBegin { step: StepKind::SecondMain, whose: PlayerRel::You }, \
             &[Effect::draw(1)], condition = Some(Condition::SourceMatches(&CHECK1)))"
        ) && kona.statics.contains("Filter::Tapped"),
        "{:?} / {:?}",
        kona.abilities,
        kona.statics
    );

    // `Phase$ Main | PhaseCount$ 1` -> `StepKind::FirstMain`.
    let first_main = read_w25c(
        "Name:FirstMain Proxy\nManaCost:1 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main | PhaseCount$ 1 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    assert_eq!(
        first_main.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::FirstMain, whose: PlayerRel::You }, \
             &[Effect::draw(1)])"
        ]
    );

    // `Phase$ Main | PhaseCount$ 2` + `CheckSVar$ Count$LifeOppsLostThisTurn` (Fireglass Mentor header).
    let mentor_trig = read_w25c(
        "Name:Mentor Trigger\nManaCost:B R\nTypes:Creature Lizard Warlock\nPT:2/1\n\
         T:Mode$ Phase | Phase$ Main | PhaseCount$ 2 | ValidPlayer$ You | CheckSVar$ X | \
         TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n\
         SVar:X:Count$LifeOppsLostThisTurn",
    );
    assert!(
        mentor_trig.abilities[0]
            .contains("Trigger::StepBegin { step: StepKind::SecondMain, whose: PlayerRel::You }")
            && mentor_trig.abilities[0].contains("Condition::OpponentLostLifeThisTurn"),
        "{:?}",
        mentor_trig.abilities
    );

    // `Phase$ Main2` on non-Phase trigger (`trigger_phase_condition`), static `Phases$ Main2`,
    // and `CastWithFlash` `Phases$ Main2`.
    let gated = read_w25c(
        "Name:SecondMain Gate\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ Drawn | ValidCard$ Card.YouCtrl | Phase$ Main2 | TriggerZones$ Battlefield | Execute$ TrigLife\n\
         SVar:TrigLife:DB$ GainLife | LifeAmount$ 1\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | Phases$ Main2\n\
         S:Mode$ CastWithFlash | ValidCard$ Sorcery | ValidSA$ Spell | Caster$ You | Phases$ Main2",
    );
    assert!(
        gated.abilities[0]
            .contains("Condition::TriggerOnly(&Condition::DuringStep(StepKind::PostcombatMain))"),
        "{:?}",
        gated.abilities
    );
    assert!(
        gated.abilities[1].contains("Condition::DuringStep(StepKind::PostcombatMain)"),
        "{:?}",
        gated.abilities
    );
    assert!(
        gated.abilities[2].contains("Condition::DuringStep(StepKind::PostcombatMain)"),
        "{:?}",
        gated.abilities
    );
}

#[test]
fn second_main_refuses_unsound_phase_and_phase_count_shapes() {
    for bad in [
        // Bare `Phase$ Main` without `PhaseCount$` means any main phase.
        "Name:AnyMain\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // `PhaseCount$ 3` is not `SecondMain`.
        "Name:ThirdMain\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main | PhaseCount$ 3 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // `Phase$ Main2` combined with `PhaseCount$` is ambiguous / redundant.
        "Name:Main2WithCount\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main2 | PhaseCount$ 2 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // `Phase$ Main1,Main2` means any main phase (including extra main phases).
        "Name:BothMains\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main1,Main2 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // Unmodeled `Phase.CheckSVar` (never silently dropped).
        "Name:BadCheckSVar\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main2 | ValidPlayer$ You | CheckSVar$ X | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw\nSVar:X:Count$YouDrewLastTurn",
    ] {
        assert!(refused_w25c(bad), "must refuse: {bad}");
    }
}

// ---------------------------------------------------------------------------
// 2. Trigger::AttackersDeclaredAtTarget (CR 508.3)
// ---------------------------------------------------------------------------

#[test]
fn attackers_declared_at_target_lowers_per_defender_and_exact_count() {
    // Attuma, Atlantean Warlord: filtered `ValidAttackers$` + `AttackedTarget$ Player`.
    let attuma = read_w25c(
        "Name:Attuma Proxy\nManaCost:2 U U\nTypes:Legendary Creature Merfolk Warrior\nPT:3/4\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player | ValidAttackers$ Creature.Merfolk+YouCtrl | \
         TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert!(
        attuma.abilities[0].contains("Trigger::AttackersDeclaredAtTarget")
            && attuma.abilities[0].contains("attacked: AttackedTargetKind::Player")
            && attuma.abilities[0].contains("exact_count: None"),
        "{:?}",
        attuma.abilities
    );

    // Rigo, Streetwise Mentor: CR 508.1b `Player,Planeswalker` with `YouCtrl` attackers
    // -> `AttackedTargetKind::OpponentOrTheirPlaneswalker`.
    let rigo = read_w25c(
        "Name:Rigo Proxy\nManaCost:1 W\nTypes:Legendary Creature Cat Citizen\nPT:2/2\n\
         T:Mode$ AttackersDeclaredOneTarget | ValidAttackers$ Creature.YouCtrl+powerLE1 | \
         AttackedTarget$ Player,Planeswalker | Execute$ TrigDraw | TriggerZones$ Battlefield\n\
         SVar:TrigDraw:DB$ Draw",
    );
    assert!(
        rigo.abilities[0].contains("Trigger::AttackersDeclaredAtTarget")
            && rigo.abilities[0]
                .contains("attacked: AttackedTargetKind::OpponentOrTheirPlaneswalker")
            && rigo.abilities[0].contains("exact_count: None"),
        "{:?}",
        rigo.abilities
    );

    // Frontier Warmonger: `Defined$ TriggeredAttackers` multi-attacker pump -> `RepeatEachObject { only_found: true }`.
    let warmonger = read_w25c(
        "Name:Frontier Warmonger\nManaCost:3 R\nTypes:Creature Human Warrior\nPT:4/4\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Opponent,Planeswalker.OppCtrl | \
         Execute$ TrigPumpAll | TriggerZones$ Battlefield\n\
         SVar:TrigPumpAll:DB$ Pump | Defined$ TriggeredAttackers | KW$ Menace",
    );
    assert!(
        warmonger.abilities[0].contains("Trigger::AttackersDeclaredAtTarget")
            && warmonger.abilities[0].contains("attackers: &Filter::CREATURE")
            && warmonger.abilities[0]
                .contains("attacked: AttackedTargetKind::OpponentOrTheirPlaneswalker")
            && warmonger.abilities[0].contains("Effect::RepeatEachObject")
            && warmonger.abilities[0].contains("only_found: true")
            && warmonger.abilities[0].contains("Effect::PumpEventObject")
            && warmonger.abilities[0].contains("KeywordSet::MENACE"),
        "{:?}",
        warmonger.abilities
    );

    // Sole-attacker (`ValidAttackersAmount$ EQ1`): `exact_count: Some(1)` + direct `PumpEventObject`
    // (Yuriko, Blade of the Mighty trigger shape).
    let sole = read_w25c(
        "Name:Sole Attacker\nManaCost:3 W\nTypes:Creature Human Samurai\nPT:2/3\n\
         T:Mode$ AttackersDeclaredOneTarget | ValidAttackers$ Creature.YouCtrl | AttackedTarget$ Player | \
         TriggerZones$ Battlefield | Execute$ TrigPump | ValidAttackersAmount$ EQ1\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttackers | KW$ Double Strike",
    );
    assert_eq!(
        sole.abilities,
        [
            "triggered!(Trigger::AttackersDeclaredAtTarget { attackers: &Filter::YOUR_CREATURE, \
             attacked: AttackedTargetKind::Player, exact_count: Some(1) }, \
             &[Effect::PumpEventObject { power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
             keywords: KeywordSet::DOUBLE_STRIKE, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn attackers_declared_at_target_lowers_defender_targets_and_preserves_plain() {
    // Whirlwind, Killer Cyclone: `TargetsWithDefinedController$ TriggeredAttackedTarget`
    // -> `TargetSpec::ObjectOfEventPlayer(&Filter::CREATURE)`.
    let whirlwind = read_w25c(
        "Name:Whirlwind, Killer Cyclone\nManaCost:2 R\nTypes:Legendary Creature Mutant Villain\nPT:2/3\n\
         K:Haste\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player | \
         ValidAttackers$ Creature.YouCtrl+ThisTurnEntered | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature | TargetsWithDefinedController$ TriggeredAttackedTarget | \
         KW$ HIDDEN CARDNAME can't block. | IsCurse$ True",
    );
    assert!(
        whirlwind.abilities[0].contains("Trigger::AttackersDeclaredAtTarget")
            && whirlwind.abilities[0].contains("KeywordSet::CANT_BLOCK")
            && whirlwind.abilities[0].contains(
                "targets = Some(TargetReq::one(TargetSpec::ObjectOfEventPlayer(&Filter::CREATURE)))"
            ),
        "{:?}",
        whirlwind.abilities
    );

    // Gornog, the Red Reaper: `DB$ Animate | RemoveCreatureTypes$ True` targeting defending player's creature.
    let gornog = read_w25c(
        "Name:Gornog Proxy\nManaCost:2 R\nTypes:Legendary Creature Minotaur Warrior\nPT:2/3\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player | \
         ValidAttackers$ Creature.Warrior+YouCtrl | TriggerZones$ Battlefield | Execute$ TrigAnimate\n\
         SVar:TrigAnimate:DB$ Animate | ValidTgts$ Creature | TargetsWithDefinedController$ TriggeredAttackedTarget | \
         Types$ Coward | RemoveCreatureTypes$ True | Duration$ Permanent",
    );
    assert!(
        gornog.abilities[0].contains("Trigger::AttackersDeclaredAtTarget")
            && gornog.abilities[0].contains("Modifier::ReplaceCreatureTypes(")
            && gornog.abilities[0].contains("Duration::Indefinitely")
            && gornog.abilities[0].contains(
                "targets = Some(TargetReq::one(TargetSpec::ObjectOfEventPlayer(&Filter::CREATURE)))"
            ),
        "{:?}",
        gornog.abilities
    );

    // Plain unfiltered `PlayerAttacked` fast path is preserved when body references attacking player.
    let plain = read_w25c(
        "Name:Plain Attacked\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ You | TriggerZones$ Battlefield | Execute$ TrigLose\n\
         SVar:TrigLose:DB$ LoseLife | Defined$ TriggeredAttackingPlayer | LifeAmount$ 1",
    );
    assert_eq!(
        plain.abilities,
        ["triggered!(Trigger::PlayerAttacked(PlayerRel::You), \
             &[Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::EventPlayer }])"]
    );
}

#[test]
fn attackers_declared_at_target_refuses_unsound_shapes() {
    for bad in [
        // `ValidAttackersAmount$ GE2` is not representable by `exact_count: Some(1)`.
        "Name:TwoOrMore\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player | ValidAttackers$ Creature.YouCtrl | \
         ValidAttackersAmount$ GE2 | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // `ValidAttackersAmount$ EQ1` with a narrowed subtype filter disagrees with CR 508.3 total-attacker `exact_count`.
        "Name:FilteredSole\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player | ValidAttackers$ Goblin.YouCtrl | \
         ValidAttackersAmount$ EQ1 | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // `AttackersDeclaredAtTarget` binds `EventPlayer` to the defender, so referencing
        // `TriggeredAttackingPlayer` (like Breena) must refuse rather than mis-bind.
        "Name:BreenaShape\nTypes:Creature\nPT:1/3\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Opponent | ValidAttackers$ Creature.YouCtrl | \
         TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw | Defined$ TriggeredAttackingPlayer",
        // Unsupported defender filter (`Opponent.lifeGTX`).
        "Name:LifeGate\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Opponent.lifeGTX | \
         TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
        // `Player,Planeswalker` when attackers are NOT exclusively `YouCtrl` cannot assume `OpponentOrTheirPlaneswalker`.
        "Name:AnyAttackerPW\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player,Planeswalker | \
         ValidAttackers$ Creature | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw",
    ] {
        assert!(refused_w25c(bad), "must refuse: {bad}");
    }
}

// ---------------------------------------------------------------------------
// 3. LKI ReplacementRule::ExileInsteadOfGraveyard (CR 603.10a, CR 614.6)
// ---------------------------------------------------------------------------

#[test]
fn lki_exile_instead_of_graveyard_covers_damage_and_valgavoth_linked_exile() {
    // Etching of Kumano: `DamagedByCard.YouCtrl;Emblem.YouCtrl` -> `DamagedByScope::SourceControllerThisTurn`.
    let etching = read_w25c(
        "Name:Etching of Kumano\nManaCost:no cost\nColors:red\nTypes:Enchantment Creature Human Shaman\nPT:2/2\n\
         K:Haste\n\
         R:Event$ Moved | ValidLKI$ Creature.DamagedByCard.YouCtrl;Emblem.YouCtrl | \
         Destination$ Graveyard | ReplaceWith$ DBExile | ActiveZones$ Battlefield\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | Destination$ Exile",
    );
    assert_eq!(
        etching.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::CREATURE, origin: Some(ZoneRef::Battlefield), owner: None, \
             controller_lki: None, damaged_by: DamagedByScope::SourceControllerThisTurn, \
             link_to_source: false, counter: None })"
        ]
    );

    // Incendiary Oracle: `ValidLKI$ Creature.DamagedBy` -> `DamagedByScope::SourceThisTurn`.
    let oracle = read_w25c(
        "Name:Incendiary Oracle\nManaCost:1 R\nTypes:Creature Human Shaman\nPT:2/2\n\
         R:Event$ Moved | ValidLKI$ Creature.DamagedBy | Destination$ Graveyard | \
         ReplaceWith$ DBExile | ActiveZones$ Battlefield\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | Destination$ Exile",
    );
    assert_eq!(
        oracle.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::CREATURE, origin: Some(ZoneRef::Battlefield), owner: None, \
             controller_lki: None, damaged_by: DamagedByScope::SourceThisTurn, \
             link_to_source: false, counter: None })"
        ]
    );

    // Valgavoth, Terror Eater: full card (including linked exile + PayLifeEqualToManaValue alt cast,
    // verified even without Oracle text!).
    let valgavoth = read_w25c(
        "Name:Valgavoth, Terror Eater\nManaCost:6 B B B\nTypes:Legendary Creature Elder Demon\nPT:9/9\n\
         K:Flying\nK:Lifelink\nK:Ward:Sac<3/Permanent.nonLand/nonland permanent>\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | \
         ValidLKI$ Card.!token+YouDontCtrl+OppOwn | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard\n\
         S:Mode$ Continuous | Condition$ PlayerTurn | MayPlay$ True | Affected$ Card.ExiledWithSource | \
         AffectedZone$ Exile | MayPlayAltManaCost$ PayLife<ConvertedManaCost>",
    );
    let valg_joined = valgavoth.abilities.join("\n");
    assert!(
        valg_joined.contains("ReplacementRule::ExileInsteadOfGraveyard")
            && valg_joined.contains("origin: None")
            && valg_joined.contains("owner: Some(PlayerRel::EachOpponent)")
            && valg_joined.contains("controller_lki: Some(PlayerRel::EachOtherPlayer)")
            && valg_joined.contains("damaged_by: DamagedByScope::Any")
            && valg_joined.contains("link_to_source: true")
            && valg_joined.contains("counter: None"),
        "{valg_joined}"
    );
    assert!(
        valg_joined.contains("Modifier::MayCastWithAltCost")
            && valg_joined.contains("zones: AltCastZones::LinkedExile")
            && valg_joined.contains("grant_zone_permission: true")
            && valg_joined.contains("allow_lands: true")
            && valg_joined.contains("price: AltCastPrice::PayLifeEqualToManaValue")
            && valg_joined.contains("condition = Some(Condition::YourTurn)"),
        "{valg_joined}"
    );
}

#[test]
fn lki_exile_instead_of_graveyard_covers_zone_owner_counter_and_self_die() {
    // Rest in Peace, Anafenza, Dryad Militant, Stone of Erech, counter-bearing exile (VOID),
    // and self-die exile (Wildwood Escort).
    let rip = read_w25c(
        "Name:Rest in Peace\nManaCost:1 W\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | ValidCard$ Card | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
    );
    assert_eq!(
        rip.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::Any, origin: None, owner: None, controller_lki: None, \
             damaged_by: DamagedByScope::Any, link_to_source: false, counter: None })"
        ]
    );

    let gisa_like = read_w25c(
        "Name:Gisa Counter Proxy\nManaCost:2 B B\nTypes:Creature Human Wizard\nPT:4/4\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | \
         ValidLKI$ Creature.!token+OppCtrl | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | \
         Destination$ Exile | WithCountersType$ VOID | WithCountersAmount$ 1",
    );
    assert_eq!(
        gisa_like.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::NONTOKEN_CREATURE, origin: Some(ZoneRef::Battlefield), owner: None, \
             controller_lki: Some(PlayerRel::EachOpponent), damaged_by: DamagedByScope::Any, \
             link_to_source: false, counter: Some(counters::VOID) })"
        ]
    );

    let dryad = read_w25c(
        "Name:Dryad Militant\nManaCost:GW\nTypes:Creature Dryad Soldier\nPT:2/1\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Instant,Sorcery | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
    );
    assert_eq!(
        dryad.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::INSTANT_OR_SORCERY, origin: None, owner: None, controller_lki: None, \
             damaged_by: DamagedByScope::Any, link_to_source: false, counter: None })"
        ]
    );

    let self_die = read_w25c(
        "Name:Wildwood Proxy\nManaCost:4 G\nTypes:Creature Elf Warrior\nPT:3/3\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Card.Self | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | Defined$ ReplacedCard",
    );
    assert_eq!(
        self_die.abilities,
        [
            "AbilityDef::Replacement(ReplacementRule::ExileInsteadOfGraveyard { \
             filter: &Filter::This, origin: Some(ZoneRef::Battlefield), owner: None, \
             controller_lki: None, damaged_by: DamagedByScope::Any, \
             link_to_source: false, counter: None })"
        ]
    );
}

#[test]
fn lki_exile_instead_of_graveyard_refuses_unsound_shapes() {
    for bad in [
        // `DamagedBy Enchanted` is NOT `SourceThisTurn` or `SourceControllerThisTurn`.
        "Name:KumanoBlessing\nTypes:Enchantment Aura\n\
         R:Event$ Moved | ValidLKI$ Creature.DamagedBy Enchanted | Destination$ Graveyard | \
         ActiveZones$ Battlefield | ReplaceWith$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | Destination$ Exile",
        // `ActiveZones$ Command` is not battlefield.
        "Name:CommandExile\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Command | Destination$ Graveyard | ValidCard$ Creature | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
        // Multi-counter exile (`WithCountersAmount$ 3`, Darigaaz).
        "Name:ThreeCounters\nTypes:Creature Dragon\nPT:7/7\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Card.Self | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | Destination$ Exile | \
         WithCountersType$ VOID | WithCountersAmount$ 3",
        // Side-effect `SubAbility$` (Kalitas / Darkness Crystal) must not be dropped.
        "Name:SubAbilityExile\nTypes:Creature\nPT:3/4\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | \
         ValidLKI$ Creature.!token+OppCtrl | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Battlefield | Destination$ Exile | SubAbility$ Gain\n\
         SVar:Gain:DB$ GainLife | LifeAmount$ 2",
        // Unmodeled origin (`Origin$ Stack`).
        "Name:StackOrigin\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Origin$ Stack | Destination$ Graveyard | \
         ValidCard$ Card | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Defined$ ReplacedCard | Origin$ Stack | Destination$ Exile",
        // Mixed-owner comma alternatives (`Artifact.OppOwn,Creature.YouOwn`).
        "Name:MixedOwners\nTypes:Enchantment\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Artifact.OppOwn,Creature.YouOwn | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
    ] {
        assert!(refused_w25c(bad), "must refuse: {bad}");
    }
}

// ---------------------------------------------------------------------------
// 4. Modifier::AdjustSpellCostByCast + SpellCastMetric (CR 601.2f)
// ---------------------------------------------------------------------------

#[test]
fn adjust_spell_cost_by_cast_covers_all_eight_metrics_reduce_and_raise() {
    // Hinata, Dawn-Crowned: `DistinctTargets` (-1 for you, +1 for opponents).
    let hinata = read_w25c(
        "Name:Hinata, Dawn-Crowned\nManaCost:1 U R W\nTypes:Legendary Creature Spirit Kirin\nPT:4/4\n\
         K:Flying\nK:Trample\n\
         S:Mode$ ReduceCost | Activator$ You | Type$ Spell | Amount$ X | Relative$ True\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ X | Relative$ True\n\
         SVar:X:TargetedObjectsDistinct$Amount",
    );
    assert_eq!(
        hinata.abilities,
        [
            "static_ability!(Filter::Any, Modifier::AdjustSpellCostByCast { \
             who: PlayerRel::You, filter: &Filter::Any, delta_per_unit: -1, \
             metric: SpellCastMetric::DistinctTargets })",
            "static_ability!(Filter::Any, Modifier::AdjustSpellCostByCast { \
             who: PlayerRel::EachOpponent, filter: &Filter::Any, delta_per_unit: 1, \
             metric: SpellCastMetric::DistinctTargets })",
        ]
    );

    // Hollow One: self-static `CardsDiscardedThisTurn` with `/Twice` -> `delta_per_unit: -2`.
    let hollow = read_w25c(
        "Name:Hollow One\nManaCost:5\nTypes:Artifact Creature Golem\nPT:4/4\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Y | Relative$ True | EffectZone$ All\n\
         K:Cycling:2\n\
         SVar:Y:PlayerCountPropertyYou$CardsDiscardedThisTurn/Twice",
    );
    assert!(
        hollow.abilities.iter().any(|a| a
            == "static_ability!(Filter::This, Modifier::AdjustSpellCostByCast { \
                who: PlayerRel::You, filter: &Filter::This, delta_per_unit: -2, \
                metric: SpellCastMetric::CardsDiscardedThisTurn }, zone = ZoneRef::Stack)"),
        "{:?}",
        hollow.abilities
    );

    // Dargo + Hierophant Bio-Titan statics: `XPaid` (-2) and `PermanentsSacrificedThisTurn(&Filter::ARTIFACT_OR_CREATURE)` (-2).
    let dargo_statics = read_w25c(
        "Name:Dargo Statics\nManaCost:6 R\nTypes:Legendary Creature Giant Pirate\nPT:7/5\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Y | EffectZone$ All | Relative$ True\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Z | EffectZone$ All\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:SVar$X/Times.2\n\
         SVar:Z:PlayerCountPropertyYou$SacrificedThisTurn Artifact,Creature/Times.2",
    );
    assert_eq!(
        dargo_statics.abilities,
        [
            "static_ability!(Filter::This, Modifier::AdjustSpellCostByCast { \
             who: PlayerRel::You, filter: &Filter::This, delta_per_unit: -2, \
             metric: SpellCastMetric::XPaid }, zone = ZoneRef::Stack)",
            "static_ability!(Filter::This, Modifier::AdjustSpellCostByCast { \
             who: PlayerRel::You, filter: &Filter::This, delta_per_unit: -2, \
             metric: SpellCastMetric::PermanentsSacrificedThisTurn(&Filter::ARTIFACT_OR_CREATURE) \
             }, zone = ZoneRef::Stack)",
        ]
    );

    // Damping Sphere static: `SpellsCastThisTurn` (+1 for each player).
    let sphere = read_w25c(
        "Name:Sphere Tax\nManaCost:2\nTypes:Artifact\n\
         S:Mode$ RaiseCost | Activator$ Player | Type$ Spell | Amount$ X | Relative$ True\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl",
    );
    assert_eq!(
        sphere.abilities,
        [
            "static_ability!(Filter::Any, Modifier::AdjustSpellCostByCast { \
             who: PlayerRel::EachPlayer, filter: &Filter::Any, delta_per_unit: 1, \
             metric: SpellCastMetric::SpellsCastThisTurn })"
        ]
    );

    // Inspired Fire: inline `Count$YouDrewThisTurn` -> `CardsDrawnThisTurn` (-1).
    let inspired = read_w25c(
        "Name:Inspired Fire\nManaCost:3 R\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Count$YouDrewThisTurn | EffectZone$ All\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4",
    );
    assert_eq!(
        inspired.abilities[0],
        "static_ability!(Filter::This, Modifier::AdjustSpellCostByCast { \
         who: PlayerRel::You, filter: &Filter::This, delta_per_unit: -1, \
         metric: SpellCastMetric::CardsDrawnThisTurn }, zone = ZoneRef::Stack)"
    );

    // Relative `LifeGainedThisTurn` (ReduceCost, EachPlayer) and `LifeLostByOpponentsThisTurn`
    // (RaiseCost, EachOpponent, `/Times.2` + spell filter `Creature`).
    let life_metrics = read_w25c(
        "Name:Life Metrics\nManaCost:3 W B\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | Activator$ Player | ValidCard$ Creature | Type$ Spell | Amount$ G | Relative$ True\n\
         S:Mode$ RaiseCost | Activator$ Opponent | ValidCard$ Instant,Sorcery | Type$ Spell | Amount$ L | Relative$ True\n\
         SVar:G:Count$LifeYouGainedThisTurn\n\
         SVar:L:Count$LifeOppsLostThisTurn/Times.2",
    );
    assert!(
        life_metrics.abilities[0]
            == "static_ability!(Filter::Any, Modifier::AdjustSpellCostByCast { \
                who: PlayerRel::EachPlayer, filter: &Filter::CREATURE, delta_per_unit: -1, \
                metric: SpellCastMetric::LifeGainedThisTurn })",
        "{:?}",
        life_metrics.abilities
    );
    assert!(
        life_metrics.abilities[1].contains("who: PlayerRel::EachOpponent")
            && life_metrics.abilities[1].contains("delta_per_unit: 2")
            && life_metrics.abilities[1]
                .contains("metric: SpellCastMetric::LifeLostByOpponentsThisTurn"),
        "{:?}",
        life_metrics.abilities
    );
}

#[test]
fn adjust_spell_cost_by_cast_refuses_unsound_and_non_paper_shapes() {
    for bad in [
        // Battlefield Thaumaturge: creature-filtered distinct targets (`TargetedObjectsDistinct$Valid ...`).
        "Name:Thaumaturge\nTypes:Creature Human Wizard\nPT:2/1\n\
         S:Mode$ ReduceCost | ValidCard$ Instant.YouCtrl,Sorcery.YouCtrl | Relative$ True | \
         Type$ Spell | Amount$ R | EffectZone$ All\n\
         SVar:R:TargetedObjectsDistinct$Valid Creature",
        // Fireball: `/Minus.1` target tax ("each target beyond the first").
        "Name:FireballTax\nTypes:Sorcery\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Inc | Relative$ True | EffectZone$ All\n\
         SVar:Inc:TargetedObjects$Amount/Minus.1\n\
         A:SP$ Draw | NumCards$ 1",
        // The Balrog, Durin's Bane: `PlayerCount$SacrificedThisTurn` counts ALL players, not only the caster.
        "Name:Balrog\nTypes:Creature Demon\nPT:7/5\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:PlayerCount$SacrificedThisTurn Permanent",
        // Thrasta: `Count$ThisTurnCast_Card` counts ALL players' spells, not `YouCtrl`.
        "Name:Thrasta\nTypes:Creature Dinosaur\nPT:7/7\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:Count$ThisTurnCast_Card/Times.3",
        // Delightful Discovery: `.OppCtrl` spells cast this turn.
        "Name:OppCast\nTypes:Sorcery\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | EffectZone$ All\n\
         SVar:X:Count$ThisTurnCast_Card.OppCtrl\n\
         A:SP$ Draw | NumCards$ 1",
        // March of Swirling Mist: `SVar$Exiled` (not `Count$xPaid`).
        "Name:MarchExile\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ Z | Relative$ True | EffectZone$ All\n\
         SVar:Z:SVar$Exiled/Times.2\n\
         SVar:Exiled:Count$CardsExiledThisWay\n\
         A:SP$ Draw | NumCards$ 1",
        // Non-`Relative` board static targeting opponents: `YouDrewThisTurn` would evaluate controller, not caster.
        "Name:NonRelOpp\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ Count$YouDrewThisTurn",
        // `DistinctTargets` without `Relative$ True`.
        "Name:NonRelTargets\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | Activator$ You | Type$ Spell | Amount$ X\n\
         SVar:X:TargetedObjectsDistinct$Amount",
        // `XPaid` on a board static (non-self zone).
        "Name:BoardXPaid\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | Activator$ You | Type$ Spell | Amount$ X | Relative$ True\n\
         SVar:X:Count$xPaid",
        // Alchemy / digital-only effect alongside Wave 25c triggers must remain refused.
        "Name:AlchemySecondMain\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main2 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ M\n\
         SVar:M:DB$ MakeCard | Conjure$ True | Name$ Mox Ruby | Zone$ Hand",
    ] {
        assert!(refused_w25c(bad), "must refuse: {bad}");
    }
}
