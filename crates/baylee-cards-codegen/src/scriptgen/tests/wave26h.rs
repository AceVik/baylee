//! Wave 26h unit tests: `tokengen.rs`, `animation.rs`, `regeneration.rs`, and
//! `damage_pair.rs` Paper-MTG unlocks and strict soundness / Paper-only refusals.

use super::*;
use crate::tokengen::{TokenLookup, read as read_token};

fn w26h_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Angel".into(),
            "Beast".into(),
            "Bird".into(),
            "Boar".into(),
            "Cat".into(),
            "Cleric".into(),
            "Construct".into(),
            "Dragon".into(),
            "Elemental".into(),
            "Elf".into(),
            "Goblin".into(),
            "Halfling".into(),
            "Human".into(),
            "Insect".into(),
            "Knight".into(),
            "Merfolk".into(),
            "Plant".into(),
            "Rogue".into(),
            "Samurai".into(),
            "Shapeshifter".into(),
            "Skeleton".into(),
            "Soldier".into(),
            "Spirit".into(),
            "Toy".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Wolf".into(),
            "Zombie".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Equipment".into(),
            "Food".into(),
            "Treasure".into(),
            "Vehicle".into(),
        ],
        enchantment: vec!["Aura".into(), "Role".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w26h(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w26h_cats();
    transcode(&parsed, &cats, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, None)
        )
    })
}

fn refused_w26h(text: &str) -> bool {
    let cats = w26h_cats();
    transcode(&parse(text), &cats, None).is_none()
}

#[test]
fn tokengen_reads_bushido_soulshift_nested_tokens_and_refuses_non_paper() {
    let cats = w26h_cats();

    let bushido = read_token(
        "Name:Samurai Token\nManaCost:no cost\nColors:white\nTypes:Creature Samurai\nPT:2/2\nK:Bushido:2",
        &cats,
    )
    .expect("Bushido token");
    assert_eq!(bushido.constant, "SAMURAI_2_2_WHITE_BUSHIDO_2");
    assert!(
        bushido
            .literal
            .contains("Trigger::BlocksOrBecomesBlockedBy")
            || bushido.literal.contains("Modifier::AddPT(2, 2)"),
        "{}",
        bushido.literal
    );

    let soulshift = read_token(
        "Name:Spirit Token\nManaCost:no cost\nColors:green\nTypes:Creature Spirit\nPT:1/1\nK:Soulshift:3",
        &cats,
    )
    .expect("Soulshift token");
    assert_eq!(soulshift.constant, "SPIRIT_1_1_GREEN_SOULSHIFT_3");
    assert!(
        soulshift.literal.contains("Trigger::Dies"),
        "{}",
        soulshift.literal
    );

    // Expanded hand-written & frozen generated nested tokens rewrite cleanly.
    let lookup = TokenLookup::held(&[
        (
            "w_4_4_angel_flying",
            "Name:Angel Token\nManaCost:no cost\nColors:white\nTypes:Creature Angel\nPT:4/4\nK:Flying",
        ),
        (
            "g_2_2_wolf",
            "Name:Wolf Token\nManaCost:no cost\nColors:green\nTypes:Creature Wolf\nPT:2/2",
        ),
        (
            "w_1_1_angel_caller",
            "Name:Cleric Token\nManaCost:no cost\nColors:white\nTypes:Creature Cleric\nPT:1/1\n\
             A:AB$ Token | Cost$ T Sac<1/CARDNAME/this token> | TokenScript$ w_4_4_angel_flying | TokenOwner$ You",
        ),
        (
            "g_2_2_wolf_caller",
            "Name:Elf Token\nManaCost:no cost\nColors:green\nTypes:Creature Elf\nPT:2/2\n\
             A:AB$ Token | Cost$ T Sac<1/CARDNAME/this token> | TokenScript$ g_2_2_wolf | TokenOwner$ You",
        ),
    ]);
    let angel_caller = lookup
        .body("w_1_1_angel_caller", &cats)
        .expect("nested hand-written ANGEL_4_4_WHITE_FLYING");
    assert!(
        angel_caller
            .literal
            .contains("&tokens::ANGEL_4_4_WHITE_FLYING"),
        "{}",
        angel_caller.literal
    );
    let wolf_caller = lookup
        .body("g_2_2_wolf_caller", &cats)
        .expect("nested frozen generated WOLF_2_2_GREEN");
    assert!(
        wolf_caller.literal.contains("&WOLF_2_2_GREEN"),
        "{}",
        wolf_caller.literal
    );

    // Non-ability CardBody cast mechanics and non-Paper Alchemy / Un-Set tokens remain refused.
    for bad in [
        "Name:Miracle Token\nManaCost:no cost\nTypes:Creature Human\nPT:1/1\nK:Miracle:1 W",
        "Name:Dash Token\nManaCost:no cost\nTypes:Creature Goblin\nPT:2/1\nK:Dash:1 R",
        "Name:Alchemy Token\nManaCost:no cost\nTypes:Creature Wizard\nPT:1/1\n\
         A:AB$ MakeCard | Cost$ T | Name$ Lightning Bolt",
        "Name:Seek Token\nManaCost:no cost\nTypes:Creature Wizard\nPT:1/1\n\
         A:AB$ Seek | Cost$ T | Type$ Land",
        "Name:Role Token\nManaCost:no cost\nTypes:Enchantment Aura Role\nK:Enchant:Creature",
    ] {
        assert!(read_token(bad, &cats).is_none(), "must refuse: {bad}");
    }
}

#[test]
fn animation_dynamic_pt_durations_and_triggered_subjects() {
    // Shape Stealer: triggered attacker/blocker P/T copy under AttackerBlockedByCreature.
    let shape_stealer = read_w26h(
        "Name:Shape Stealer\nManaCost:U U\nTypes:Creature Shapeshifter\nPT:1/1\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature | Execute$ TrigAnimateBlocker\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Creature | ValidBlocker$ Card.Self | Execute$ TrigAnimateAttacker | Secondary$ True\n\
         SVar:TrigAnimateBlocker:DB$ Animate | Defined$ Self | Power$ BlockerP | Toughness$ BlockerT\n\
         SVar:TrigAnimateAttacker:DB$ Animate | Defined$ Self | Power$ AttackerP | Toughness$ AttackerT\n\
         SVar:BlockerP:TriggeredBlockerLKICopy$CardPower\n\
         SVar:BlockerT:TriggeredBlockerLKICopy$CardToughness\n\
         SVar:AttackerP:TriggeredAttackerLKICopy$CardPower\n\
         SVar:AttackerT:TriggeredAttackerLKICopy$CardToughness",
    );
    assert_eq!(shape_stealer.abilities.len(), 1);
    assert!(
        shape_stealer.abilities[0].contains(
            "Effect::SetPTFilter { filter: &Filter::This, power: Amount::EventPower, toughness: Amount::EventLastToughness, duration: Duration::UntilEndOfTurn }"
        ),
        "{}",
        shape_stealer.abilities[0]
    );

    // Durations: UntilYourNextUpkeep and UntilTheEndOfYourNextUpkeep.
    let upkeep = read_w26h(
        "Name:Upkeep Sculptor\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Artifact | Power$ 4 | Toughness$ 4 | Types$ Creature | Duration$ UntilYourNextUpkeep",
    );
    assert!(
        upkeep.abilities[0].contains("Duration::UntilYourNextUpkeep"),
        "{}",
        upkeep.abilities[0]
    );

    let end_upkeep = read_w26h(
        "Name:Upkeep Weaver\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Artifact | Power$ 3 | Toughness$ 3 | Types$ Creature | Duration$ UntilTheEndOfYourNextUpkeep",
    );
    assert!(
        end_upkeep.abilities[0].contains("Duration::UntilEndOfYourNextUpkeep"),
        "{}",
        end_upkeep.abilities[0]
    );

    // Dynamic P/T expressions: CountersOnSource, PartySize, GreatestPowerAmong, arithmetic modifiers.
    let counters_pt = read_w26h(
        "Name:Charge Automaton\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ X | Toughness$ X | Types$ Creature\n\
         SVar:X:Count$CardCounters.CHARGE",
    );
    assert!(
        counters_pt.abilities[0].contains("Amount::CountersOnSource(CounterKind::Charge)"),
        "{}",
        counters_pt.abilities[0]
    );

    let party_pt = read_w26h(
        "Name:Party Banner\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 1 | Defined$ Self | Power$ X | Toughness$ Y | Types$ Creature\n\
         SVar:X:Count$Party\n\
         SVar:Y:SVar$X/Times.2",
    );
    assert!(
        party_pt.abilities[0].contains("Amount::PartySize(PlayerRel::You)")
            && party_pt.abilities[0]
                .contains("Amount::Times { base: &Amount::PartySize(PlayerRel::You), factor: 2 }"),
        "{}",
        party_pt.abilities[0]
    );

    let greatest_pt = read_w26h(
        "Name:Apex Idol\nManaCost:4\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ X | Toughness$ Y | Types$ Creature\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardPower\n\
         SVar:Y:SVar$X/HalfUp",
    );
    assert!(
        greatest_pt.abilities[0].contains("Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE)")
            && greatest_pt.abilities[0].contains("round_up: true"),
        "{}",
        greatest_pt.abilities[0]
    );

    // Triggered subject on Taps + CountersOnSource dynamic P/T.
    let tap_animate = read_w26h(
        "Name:Clockwork Awakener\nManaCost:3\nTypes:Enchantment\n\
         T:Mode$ Taps | ValidCard$ Artifact.nonCreature+YouCtrl | Execute$ TrigAnim\n\
         SVar:TrigAnim:DB$ Animate | Defined$ TriggeredCard | Power$ X | Toughness$ X | Types$ Creature\n\
         SVar:X:Count$CardCounters.CHARGE",
    );
    assert!(
        tap_animate.abilities[0].contains("Effect::SetPTFilter { filter: &Filter::Reference")
            && tap_animate.abilities[0].contains("Amount::CountersOnSource(CounterKind::Charge)"),
        "{}",
        tap_animate.abilities[0]
    );

    // Strict refusals: Alchemy Perpetual duration, ungrounded Targeted$CardPower without target, and zero factor.
    for bad in [
        "Name:BadPerp\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Artifact | Power$ 3 | Toughness$ 3 | Types$ Creature | Duration$ Perpetual",
        "Name:BadUntargetedTargetPower\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 1 | Defined$ Self | Power$ X | Toughness$ X | Types$ Creature\n\
         SVar:X:Targeted$CardPower",
        "Name:BadMinusZeroFactor\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 1 | Defined$ Self | Power$ X | Toughness$ X | Types$ Creature\n\
         SVar:X:SVar$Y/Times.0\n\
         SVar:Y:Count$CardPower",
    ] {
        assert!(refused_w26h(bad), "must refuse: {bad}");
    }
}

#[test]
fn regeneration_exile_damaged_this_way_and_refusals() {
    // Anger of the Gods shape: DamageAll + RememberDamaged$ True + ReplaceDyingDefined$ Remembered + Cleanup.
    let anger = read_w26h(
        "Name:Anger of the Gods\nManaCost:1 R R\nTypes:Sorcery\n\
         A:SP$ DamageAll | ValidCards$ Creature | NumDmg$ 3 | RememberDamaged$ True | ReplaceDyingDefined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        anger.abilities[0].contains("Effect::DealDamageEach")
            && anger.abilities[0].contains("Effect::ExileDamagedThisWayIfDies"),
        "{}",
        anger.abilities[0]
    );

    // Spikefield Hazard shape: DealDamage + RememberDamaged$ True + ReplaceDyingDefined$ Remembered + Cleanup.
    let hazard = read_w26h(
        "Name:Spikefield Hazard\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 1 | RememberDamaged$ True | ReplaceDyingDefined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        hazard.abilities[0].contains("Effect::ExileDamagedThisWayIfDies"),
        "{}",
        hazard.abilities[0]
    );

    // Pillar of Flame / Red Sun's Zenith shape: DealDamage (spell) + RememberDamaged$ True + ReplaceDyingDefined$ Remembered.Creature + trailing shuffle/cleanup.
    let pillar = read_w26h(
        "Name:Pillar of Flame\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | RememberDamaged$ True | ReplaceDyingDefined$ Remembered.Creature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        pillar.abilities[0].contains("Effect::RepeatEachObject")
            && pillar.abilities[0]
                .contains("Effect::ExileIfDiesThisTurn { target: TargetSpec::EventObject }")
            && pillar
                .statics
                .contains("Filter::DealtDamageBySourceThisTurn"),
        "{} | {}",
        pillar.abilities[0],
        pillar.statics
    );

    // Unnatural Aggression shape: ReplaceDyingDefined$ ThisTargetedCard.
    let this_targeted = read_w26h(
        "Name:Searing Brand\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | ReplaceDyingDefined$ ThisTargetedCard",
    );
    assert!(
        this_targeted.abilities[0].contains(
            "Effect::ExileIfDiesThisTurn { target: TargetSpec::Object(&Filter::CREATURE) }"
        ),
        "{}",
        this_targeted.abilities[0]
    );

    // Strict refusals:
    // 1. Standalone Remembered.Creature without RememberDamaged$ True.
    // 2. Burn from Within shape where a later sub-ability consumes Remembered (Pump | Defined$ Remembered).
    // 3. ReplaceDyingCondition$ present.
    // 4. Non-self DamageSource$.
    // 5. Remembered.Creature on a permanent activated ability (multi-activation bleed).
    for bad in [
        "Name:NoRemember\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | ReplaceDyingDefined$ Remembered.Creature",
        "Name:BurnFromWithinShape\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | RememberDamaged$ True | ReplaceDyingDefined$ Remembered | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Remembered | KW$ HIDDEN CARDNAME loses indestructible. | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
        "Name:CondReplace\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | RememberDamaged$ True | ReplaceDyingDefined$ Remembered | ReplaceDyingCondition$ Bargain | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        "Name:NonSelfSource\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | DamageSource$ Targeted | NumDmg$ 2 | RememberDamaged$ True | ReplaceDyingDefined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        "Name:PermRememberedCreature\nManaCost:2 R\nTypes:Creature Wizard\nPT:1/1\n\
         A:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | RememberDamaged$ True | ReplaceDyingDefined$ Remembered.Creature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ] {
        assert!(refused_w26h(bad), "must refuse: {bad}");
    }
}

#[test]
fn regeneration_combat_damage_toughness_grant_flash_and_at_eot() {
    // Bill the Pony shape: Mode$ CombatDamageToughness on Creature.YouCtrl.
    let bill = read_w26h(
        "Name:Toughness Striker\nManaCost:2 W\nTypes:Creature Halfling\nPT:1/4\n\
         A:AB$ Effect | Cost$ 1 | StaticAbilities$ Tough | SpellDescription$ Until end of turn, each creature you control assigns combat damage equal to its toughness rather than its power.\n\
         SVar:Tough:Mode$ CombatDamageToughness | ValidCard$ Creature.YouCtrl",
    );
    assert!(
        bill.abilities[0].contains("Modifier::AssignCombatDamageUsingToughness"),
        "{}",
        bill.abilities[0]
    );

    // Greater toughness conditional variant (`+powerLTtoughness`).
    let greater_tough = read_w26h(
        "Name:Bulwark Stance\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ Tough\n\
         SVar:Tough:Mode$ CombatDamageToughness | ValidCard$ Creature.YouCtrl+powerLTtoughness",
    );
    assert!(
        greater_tough.abilities[0]
            .contains("Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater"),
        "{}",
        greater_tough.abilities[0]
    );

    // Wave 14a GrantFlash with filtered ValidCard$ (Goblin) and Command zone.
    let flash_goblins = read_w26h(
        "Name:Goblin Flash Shape\nManaCost:R\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ FlashGob\n\
         SVar:FlashGob:Mode$ CastWithFlash | ValidCard$ Goblin | ValidSA$ Spell | Caster$ You | EffectZone$ Command",
    );
    assert!(
        flash_goblins.abilities[0]
            .contains("Modifier::GrantFlash { who: PlayerRel::You, filter: &FLASH1 }")
            && flash_goblins.statics.contains("subtypes::creature::GOBLIN"),
        "{} | {}",
        flash_goblins.abilities[0],
        flash_goblins.statics
    );

    // Winding Canyons shape: all-5 castable AffectedZone$ + Creature -> SpellsHaveFlash(Some(TypeSet::CREATURE)).
    let winding_canyons = read_w26h(
        "Name:Winding Canyons\nManaCost:no cost\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ C\n\
         A:AB$ Effect | Cost$ 2 T | StaticAbilities$ GiveFlash\n\
         SVar:GiveFlash:Mode$ CastWithFlash | ValidCard$ Creature | ValidSA$ Spell | Caster$ You | AffectedZone$ Exile,Graveyard,Hand,Library,Command",
    );
    assert!(
        winding_canyons.abilities[1].contains("Modifier::SpellsHaveFlash(Some(TypeSet::CREATURE))"),
        "{}",
        winding_canyons.abilities[1]
    );

    // Preserves legacy fast path for bare Card (`SpellsHaveFlash(None)`).
    let flash_all = read_w26h(
        "Name:Quicken All Shape\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ FlashAll\n\
         SVar:FlashAll:Mode$ CastWithFlash | ValidCard$ Card | ValidSA$ Spell | Caster$ You | EffectZone$ Command",
    );
    assert!(
        flash_all.abilities[0].contains("Modifier::SpellsHaveFlash(None)"),
        "{}",
        flash_all.abilities[0]
    );

    // Narrow AffectedZone$ (Hand only) stays refused.
    assert!(refused_w26h(
        "Name:HandOnlyFlash\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ FlashHand\n\
         SVar:FlashHand:Mode$ CastWithFlash | ValidCard$ Creature | ValidSA$ Spell | Caster$ You | AffectedZone$ Hand"
    ));
}

#[test]
fn regeneration_at_eot_and_animate_source_grants() {
    // AtEOT$ on Pump (self-sacrifice like Attack-in-the-Box / Pyric Salamander & YouCtrl target like Fatal Frenzy),
    // Animate (Chimeric Coils & Ayara, Furnace Queen), and Token (YourExile / YourSacrificeUpkeep).
    let attack_box = read_w26h(
        "Name:Attack-in-the-Box\nManaCost:3\nTypes:Artifact Creature Toy\nPT:2/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigPump | OptionalDecider$ You\n\
         SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +4 | AtEOT$ Sacrifice",
    );
    assert!(
        attack_box.abilities[0]
            .contains("Effect::AtNextEndStep { effects: &[Effect::SacrificeEvent] }"),
        "{}",
        attack_box.abilities[0]
    );

    let fatal_frenzy = read_w26h(
        "Name:Fatal Frenzy\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +X | KW$ Trample | AtEOT$ Sacrifice\n\
         SVar:X:Targeted$CardPower",
    );
    assert!(
        fatal_frenzy.abilities[0]
            .contains("Effect::AtNextEndStep { effects: &[Effect::SacrificeEvent] }"),
        "{}",
        fatal_frenzy.abilities[0]
    );

    let chimeric_coils = read_w26h(
        "Name:Chimeric Coils\nManaCost:1\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ X 1 | Defined$ Self | Power$ X | Toughness$ X | Types$ Creature,Artifact,Construct | RemoveCreatureTypes$ True | AtEOT$ Sacrifice | Duration$ Permanent\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        chimeric_coils.abilities[0]
            .contains("Effect::AtNextEndStep { effects: &[Effect::SacrificeEvent] }"),
        "{}",
        chimeric_coils.abilities[0]
    );

    let token_your_exile = transcode(
        &parse(
            "Name:Spark Caller\nManaCost:2 R\nTypes:Instant\n\
             A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | AtEOT$ YourExile",
        ),
        &w26h_cats(),
        Some(&tokens()),
    )
    .expect("Token AtEOT$ YourExile");
    assert!(
        token_your_exile.abilities[0].contains("DelayedStep::EndStep")
            && token_your_exile.abilities[0].contains("whose: Some(PlayerRel::You)")
            && token_your_exile.abilities[0].contains("TriggerRemember::EachCreatedToken"),
        "{}",
        token_your_exile.abilities[0]
    );

    // Source-only Animate staticAbilities$ for CantBlock / MustAttack / AssignNoCombatDamage.
    let anim_cant_block = read_w26h(
        "Name:Reckless Construct\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 1 | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature | staticAbilities$ NoBlk\n\
         SVar:NoBlk:Mode$ CantBlock | ValidCard$ Card.Self",
    );
    assert!(
        anim_cant_block.abilities[0].contains("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"),
        "{}",
        anim_cant_block.abilities[0]
    );

    // Soundness refusals: non-Self triggered Pump AtEOT$, Aura Pump AtEOT$, opponent-targetable Pump AtEOT$ Sacrifice.
    for bad in [
        "Name:WrongTrigPump\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ Attacks | ValidCard$ Creature.YouCtrl | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredCard | NumAtt$ +2 | AtEOT$ Sacrifice",
        "Name:AuraPumpAtEot\nManaCost:1 R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ Pump | Cost$ R | Defined$ Enchanted | NumAtt$ +1 | AtEOT$ Sacrifice",
        "Name:OppPumpSac\nManaCost:1 R\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ Pump | Cost$ T | ValidTgts$ Creature | KW$ Haste | AtEOT$ Sacrifice",
    ] {
        assert!(refused_w26h(bad), "must refuse: {bad}");
    }
}

#[test]
fn damage_pair_party_conditions_and_subjects() {
    // Synchronized Spellcraft: Count$Party -> Amount::PartySize(PlayerRel::You).
    let spellcraft = read_w26h(
        "Name:Synchronized Spellcraft\nManaCost:4 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | NumDmg$ X | Defined$ TargetedController\n\
         SVar:X:Count$Party\n\
         Oracle:Synchronized Spellcraft deals 4 damage to target creature and X damage to that creature's controller, where X is the number of creatures in your party.",
    );
    assert!(
        spellcraft.abilities[0].contains(
            "Effect::DamagePermanentAndController { target: TargetSpec::Object(&Filter::CREATURE), permanent_amount: Amount::Fixed(4), controller_amount: Amount::PartySize(PlayerRel::You) }"
        ),
        "{}",
        spellcraft.abilities[0]
    );

    // Conditioned damage pair: Delirium and battlefield ConditionPresent$ Equipment.YouCtrl.
    let delirium_pair = read_w26h(
        "Name:Cinder Delirium\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | DamageMap$ True | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ TargetedController | Condition$ Delirium | NumDmg$ 2 | SubAbility$ DBRes\n\
         SVar:DBRes:DB$ DamageResolve\n\
         Oracle:Cinder Delirium deals 3 damage to target creature. Delirium — If there are four or more card types among cards in your graveyard, instead Cinder Delirium deals 3 damage to that creature and 2 damage to that creature's controller.",
    );
    assert!(
        delirium_pair.abilities[0]
            .contains("Effect::IfCondition { condition: Condition::GraveyardTypesAtLeast(4), then: &[Effect::DamagePermanentAndController"),
        "{}",
        delirium_pair.abilities[0]
    );

    let equip_pair = read_w26h(
        "Name:Armed Scorch\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Artifact | NumDmg$ 3 | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ TargetedController | ConditionPresent$ Equipment.YouCtrl | ConditionCompare$ GE1 | NumDmg$ 2\n\
         Oracle:Armed Scorch deals 3 damage to target artifact and 2 damage to that artifact's controller if you control an Equipment.",
    );
    assert!(
        equip_pair.abilities[0].contains(
            "Condition::ControlCount(&Filter::HasSubtype(subtypes::artifact::EQUIPMENT), 1)"
        ) && equip_pair.abilities[0].contains("Effect::DamagePermanentAndController"),
        "{}",
        equip_pair.abilities[0]
    );

    // TriggeredCard + TriggeredCardController pair on ChangesZone.
    let etb_pair = read_w26h(
        "Name:Spiteful Furnace\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredCard | NumDmg$ 1 | SubAbility$ DBPlayer\n\
         SVar:DBPlayer:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 1\n\
         Oracle:Whenever a creature enters, Spiteful Furnace deals 1 damage to that creature and 1 damage to that creature's controller.",
    );
    assert!(
        etb_pair.abilities[0].contains(
            "Effect::DamagePermanentAndController { target: TargetSpec::EventObject, permanent_amount: Amount::Fixed(1), controller_amount: Amount::Fixed(1) }"
        ),
        "{}",
        etb_pair.abilities[0]
    );

    // Refusals: OppCtrl in ConditionPresent, non-self DamageSource$, unknown Condition$.
    for bad in [
        "Name:BadCondOpp\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ TargetedController | ConditionPresent$ Creature.OppCtrl | ConditionCompare$ GE1 | NumDmg$ 2\n\
         Oracle:BadCondOpp deals 3 damage to target creature and 2 damage to that creature's controller.",
        "Name:BadCondName\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ TargetedController | Condition$ UnknownMagic | NumDmg$ 2\n\
         Oracle:BadCondName deals 3 damage to target creature and 2 damage to that creature's controller.",
    ] {
        assert!(refused_w26h(bad), "must refuse: {bad}");
    }
}
