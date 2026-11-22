//! Wave 26b codegen tests: Wave 14 `Amount::` & `ThisWay::` `SVar` readers in
//! `atoms.rs`, plus `ActivationLimit::OncePerPermanent` and
//! `CostReduction::OwnManaCostIfEnteredThisTurn` (`PowerUp$ True`) and Wave 14
//! activation limits/reductions/conditions in `activated.rs`.

use super::*;
use crate::scriptgen::atoms::{amount, pump_amount};

fn svars_of(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn wave26b_greatest_power_and_greatest_mana_value_amounts() {
    // Rishkar's Expertise / Soul's Majesty draw shape (`Count$Valid Creature.YouCtrl$GreatestCardPower`).
    let draw_greatest = read(
        "Name:Wildspeaker Draw\nManaCost:4 G\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardPower",
    );
    assert!(
        draw_greatest.abilities[0].contains(
            "Effect::DrawCards { amount: Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE) }"
        ),
        "{:?}",
        draw_greatest.abilities
    );

    // Bouncer's Beatdown-style DealDamage + PutCounter + scaled `/Twice` + `SVar$` chain.
    let dmg_greatest = read(
        "Name:Beatdown Bolt\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardPower",
    );
    assert!(
        dmg_greatest.abilities[0].contains(
            "Effect::DealDamage { amount: Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE), \
             target: TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER) }"
        ),
        "{:?}",
        dmg_greatest.abilities
    );

    // Pump +X/+X and -X/-X with `GreatestPowerAmong` and `GreatestManaValueAmong`.
    let pump_greatest = read(
        "Name:Apex Surge\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ -Y\n\
         SVar:X:Count$Valid Creature.YouCtrl+Other$GreatestCardPower\n\
         SVar:Y:Count$Valid Artifact.YouCtrl$GreatestCardManaCost",
    );
    assert!(
        pump_greatest.abilities[0]
            .contains("power: Amount::GreatestPowerAmong(&Filter::ANOTHER_CREATURE_YOU_CONTROL)")
            && pump_greatest.abilities[0]
                .contains("toughness: Amount::Negated(&Amount::GreatestManaValueAmong(&Filter::YOUR_ARTIFACT))"),
        "{:?}",
        pump_greatest.abilities
    );

    // Greatest mana value variants: Permanent.YouCtrl, Creature.YouCtrl, Graveyard, BattlefieldOrCommand.
    let mv_cases = [
        (
            "Count$Valid Permanent.YouCtrl$GreatestCardManaCost",
            "Amount::GreatestManaValueAmong(&Filter::ControlledByYou)",
        ),
        (
            "Count$Valid Creature.YouCtrl$GreatestCardManaCost",
            "Amount::GreatestManaValueAmong(&Filter::YOUR_CREATURE)",
        ),
        (
            "Count$Valid Permanent.nonLand+YouCtrl$GreatestCardManaCost/Plus.1",
            "Amount::Plus { base: &Amount::GreatestManaValueAmong(&Filter::And(&[Filter::NONLAND, Filter::ControlledByYou])), offset: 1 }",
        ),
        (
            "Count$ValidBattlefield,Command Card.IsCommander+YouOwn$GreatestCardManaCost",
            "Amount::GreatestManaValueAmong(&Filter::And(&[Filter::IsCommander, Filter::OwnedByYou, Filter::InZone(ZoneRef::BattlefieldOrCommand)]))",
        ),
        (
            "Count$ValidGraveyard Creature.YouOwn$GreatestCardPower",
            "Amount::GreatestPowerAmong(&Filter::And(&[Filter::CREATURE, Filter::OwnedByYou, Filter::InZone(ZoneRef::Graveyard)]))",
        ),
        (
            "Count$ValidGraveyard Card.YouOwn$GreatestCardManaCost",
            "Amount::GreatestManaValueAmong(&Filter::And(&[Filter::OwnedByYou, Filter::InZone(ZoneRef::Graveyard)]))",
        ),
        (
            "Count$Valid Creature.YouDontCtrl$GreatestCardPower",
            "Amount::GreatestPowerAmong(&Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::ControlledByYou)]))",
        ),
    ];
    for (def, expected) in mv_cases {
        let svars = svars_of(&[("X", def)]);
        assert_eq!(
            amount("X", &svars, false).as_deref(),
            Some(expected),
            "{def}"
        );
    }
}

#[test]
fn wave26b_party_size_real_paper_cards_and_conditions() {
    // Malakir Blood-Priest: ETB each opponent loses X life and you gain X life (`Count$Party`).
    let priest = read(
        "Name:Malakir Blood-Priest\nManaCost:1 B\nTypes:Creature Vampire Cleric\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigLoseLife\n\
         SVar:TrigLoseLife:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$Party",
    );
    assert!(
        priest.abilities[0].contains(
            "Effect::LoseLife { amount: Amount::PartySize(PlayerRel::You), target: PlayerRel::Opponent }"
        ) && priest.abilities[0]
            .contains("Effect::GainLife { amount: Amount::PartySize(PlayerRel::You) }"),
        "{:?}",
        priest.abilities
    );

    // Thundering Sparkmage: ETB deals X damage (`Count$Party`).
    let sparkmage = read(
        "Name:Thundering Sparkmage\nManaCost:3 R\nTypes:Creature Human Wizard\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDealDamage\n\
         SVar:TrigDealDamage:DB$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ X\n\
         SVar:X:Count$Party",
    );
    assert!(
        sparkmage.abilities[0].contains("amount: Amount::PartySize(PlayerRel::You)"),
        "{:?}",
        sparkmage.abilities
    );

    // Practiced Tactics: `Count$Party/Twice` DealDamage.
    let tactics = read(
        "Name:Practiced Tactics\nManaCost:W\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.attacking,Creature.blocking | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | Defined$ Targeted | NumDmg$ X\n\
         SVar:X:Count$Party/Twice",
    );
    assert!(
        tactics.abilities[0].contains(
            "amount: Amount::Times { base: &Amount::PartySize(PlayerRel::You), factor: 2 }"
        ),
        "{:?}",
        tactics.abilities
    );

    // Kabira Outrider (+X/+X) & Drana's Silencer (-X/-X) & Cascade Seer (Scry X).
    let outrider = read(
        "Name:Kabira Outrider\nManaCost:3 W\nTypes:Creature Human Warrior\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$Party",
    );
    assert!(
        outrider.abilities[0].contains(
            "Effect::PumpTarget { power: Amount::PartySize(PlayerRel::You), \
             toughness: Amount::PartySize(PlayerRel::You)"
        ),
        "{:?}",
        outrider.abilities
    );

    let silencer = read(
        "Name:Drana's Silencer\nManaCost:5 B\nTypes:Creature Vampire Cleric\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature.OppCtrl | IsCurse$ True | NumAtt$ -X | NumDef$ -X\n\
         SVar:X:Count$Party",
    );
    assert!(
        silencer.abilities[0].contains(
            "power: Amount::Negated(&Amount::PartySize(PlayerRel::You)), \
             toughness: Amount::Negated(&Amount::PartySize(PlayerRel::You))"
        ),
        "{:?}",
        silencer.abilities
    );

    let seer = read(
        "Name:Cascade Seer\nManaCost:3 U\nTypes:Creature Merfolk Wizard\nPT:3/3\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ TrigScry\n\
         SVar:TrigScry:DB$ Scry | ScryNum$ X\n\
         SVar:X:Count$Party",
    );
    assert!(
        seer.abilities[0].contains("Effect::Scry { amount: Amount::PartySize(PlayerRel::You) }"),
        "{:?}",
        seer.abilities
    );
}

#[test]
fn wave26b_party_size_counter_tax_and_full_party_condition() {
    // Concerted Defense: `UnlessCost$ Y` with `Count$Party/Plus.1`.
    let defense = read(
        "Name:Concerted Defense\nManaCost:U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.nonCreature | UnlessCost$ Y\n\
         SVar:Y:Count$Party/Plus.1",
    );
    assert!(
        defense.abilities[0]
            .contains("Amount::Plus { base: &Amount::PartySize(PlayerRel::You), offset: 1 }"),
        "{:?}",
        defense.abilities
    );

    // Full-party `CheckSVar$ X | SVarCompare$ EQ4` condition via `activated.rs::svar_condition`.
    let full_party = read(
        "Name:Full Party Adept\nManaCost:2 W\nTypes:Creature Human Cleric\nPT:2/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ X | SVarCompare$ EQ4\n\
         SVar:X:Count$Party",
    );
    assert!(
        full_party.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::PartySize(PlayerRel::You), 4)")
            && full_party
                .statics
                .contains("Condition::AmountAtLeast(Amount::PartySize(PlayerRel::You), 5)"),
        "{:?}\n{}",
        full_party.abilities,
        full_party.statics
    );
}

#[test]
fn wave26b_spells_cast_cards_drawn_and_total_power_amounts() {
    // SpellsCastThisTurn (You, Opponent, EachPlayer), CardsDrawnThisTurn (You, Opponent, Chosen),
    // and TotalPowerAmong across effects and ETB counters.
    let spells_dmg = read(
        "Name:Storm Flare\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl",
    );
    assert!(
        spells_dmg.abilities[0].contains("amount: Amount::SpellsCastThisTurn(PlayerRel::You)"),
        "{:?}",
        spells_dmg.abilities
    );

    let drawn_pump = read(
        "Name:Scholar's Might\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$YouDrewThisTurn",
    );
    assert!(
        drawn_pump.abilities[0].contains(
            "power: Amount::CardsDrawnThisTurn(PlayerRel::You), \
             toughness: Amount::CardsDrawnThisTurn(PlayerRel::You)"
        ),
        "{:?}",
        drawn_pump.abilities
    );

    let total_power_gain = read(
        "Name:Herd Vitality\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$CardPower",
    );
    assert!(
        total_power_gain.abilities[0].contains(
            "Effect::GainLife { amount: Amount::TotalPowerAmong(&Filter::YOUR_CREATURE) }"
        ),
        "{:?}",
        total_power_gain.abilities
    );

    let total_power_etb = read(
        "Name:Pack Hydra\nManaCost:4 G\nTypes:Creature Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         SVar:X:Count$Valid Creature.YouCtrl$CardPower",
    );
    assert_eq!(
        total_power_etb.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
             amount: Amount::TotalPowerAmong(&Filter::YOUR_CREATURE) }"]
    );

    let unit_cases = [
        (
            "Count$ThisTurnCast_Card.OppCtrl",
            "Amount::SpellsCastThisTurn(PlayerRel::Opponent)",
        ),
        (
            "Count$ThisTurnCast_Card/Times.3",
            "Amount::Times { base: &Amount::SpellsCastThisTurn(PlayerRel::EachPlayer), factor: 3 }",
        ),
        (
            "Count$YouDrewThisTurn/Twice",
            "Amount::Times { base: &Amount::CardsDrawnThisTurn(PlayerRel::You), factor: 2 }",
        ),
        (
            "PlayerCountOpponents$CardsDrawn",
            "Amount::CardsDrawnThisTurn(PlayerRel::Opponent)",
        ),
        (
            "TargetedPlayer$CardsDrawn",
            "Amount::CardsDrawnThisTurn(PlayerRel::Chosen)",
        ),
        (
            "Count$Valid Creature.attacking$CardPower",
            "Amount::TotalPowerAmong(&Filter::ATTACKING_CREATURE)",
        ),
        (
            "Count$TotalPower Creature.YouCtrl",
            "Amount::TotalPowerAmong(&Filter::YOUR_CREATURE)",
        ),
        (
            "Count$Valid Creature.OppCtrl$CardPower/HalfDown",
            "Amount::Half { base: &Amount::TotalPowerAmong(&Filter::OPPONENT_CREATURE), round_up: false }",
        ),
    ];
    for (def, expected) in unit_cases {
        let svars = svars_of(&[("X", def)]);
        assert_eq!(
            amount("X", &svars, false).as_deref(),
            Some(expected),
            "{def}"
        );
    }

    // Filtered spell casts MUST NOT approximate to unfiltered `SpellsCastThisTurn`.
    for bad_def in [
        "Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
        "Count$ThisTurnCast_Creature.YouCtrl",
        "Count$ThisTurnCast_Card.nonCreature+YouCtrl",
        "Count$Party/Times.0",
        "Count$Valid Creature.YouCtrl$GreatestCardToughness",
    ] {
        let svars = svars_of(&[("X", bad_def)]);
        assert_eq!(amount("X", &svars, false), None, "must refuse `{bad_def}`");
    }
}

#[test]
fn wave26b_sacrificed_stats_chosen_number_and_this_way_readers() {
    // Sacrificed stats: TotalToughness, TotalManaValue, DistinctColors (including Evolving Door `/Plus.1`).
    let cases = [
        ("Sacrificed$TotalCardPower", "Amount::SacrificedTotalPower"),
        (
            "Sacrificed$TotalCardToughness",
            "Amount::SacrificedTotalToughness",
        ),
        (
            "Sacrificed$TotalCardManaCost",
            "Amount::SacrificedTotalManaValue",
        ),
        (
            "Sacrificed$CardNumColors",
            "Amount::SacrificedDistinctColors",
        ),
        (
            "Sacrificed$CardNumColors/Plus.1",
            "Amount::Plus { base: &Amount::SacrificedDistinctColors, offset: 1 }",
        ),
        (
            "Sacrificed$CardManaCost/Plus.1",
            "Amount::Plus { base: &Amount::SacrificedManaValue, offset: 1 }",
        ),
        ("Count$ChosenNumber", "Amount::ResolutionChosenNumber"),
        (
            "Count$ChosenNumber/Twice",
            "Amount::Times { base: &Amount::ResolutionChosenNumber, factor: 2 }",
        ),
        (
            "PreventedDamage",
            "Amount::ThisWay(ThisWay::DamagePrevented)",
        ),
        (
            "Count$PreventedDamage/Plus.1",
            "Amount::Plus { base: &Amount::ThisWay(ThisWay::DamagePrevented), offset: 1 }",
        ),
        (
            "Count$SacrificedThisWay",
            "Amount::ThisWay(ThisWay::SacrificedCount)",
        ),
        (
            "Sacrificed$Amount",
            "Amount::ThisWay(ThisWay::SacrificedCount)",
        ),
    ];
    for (def, expected) in cases {
        let svars = svars_of(&[("X", def)]);
        assert_eq!(
            amount("X", &svars, false).as_deref(),
            Some(expected),
            "{def}"
        );
    }

    // Direct literal `PreventedDamage` (Temper / Test of Faith / Candles' Glow / Vengeful Archon).
    let empty = BTreeMap::new();
    assert_eq!(
        amount("PreventedDamage", &empty, false).as_deref(),
        Some("Amount::ThisWay(ThisWay::DamagePrevented)")
    );
    assert_eq!(
        pump_amount("+PreventedDamage", &empty, false).as_deref(),
        Some("Amount::ThisWay(ThisWay::DamagePrevented)")
    );

    // Die Young `-X/-X` with `SVar:X:Count$ChosenNumber` -> `Amount::Negated(&Amount::ResolutionChosenNumber)`.
    let chosen_svars = svars_of(&[("X", "Count$ChosenNumber")]);
    assert_eq!(
        pump_amount("-X", &chosen_svars, false).as_deref(),
        Some("Amount::Negated(&Amount::ResolutionChosenNumber)")
    );

    // Inline `Count$ChosenNumber` requires a `ChooseNumber` SVar and zero `SetChosenNumber$`.
    let with_choose = svars_of(&[("DBChoose", "DB$ ChooseNumber | Max$ 5")]);
    assert_eq!(
        amount("Count$ChosenNumber", &with_choose, false).as_deref(),
        Some("Amount::ResolutionChosenNumber")
    );
    assert_eq!(
        amount("Count$ChosenNumber", &empty, false),
        None,
        "inline Count$ChosenNumber without ChooseNumber must refuse"
    );

    // `SetChosenNumber$` (Wildgrowth Archaic / Communal Brewing) must NEVER resolve as `ResolutionChosenNumber`.
    let with_set_chosen = svars_of(&[
        ("TrigEffect", "DB$ Effect | SetChosenNumber$ Y"),
        ("X", "Count$ChosenNumber"),
    ]);
    assert_eq!(amount("X", &with_set_chosen, false), None);
    assert_eq!(amount("Count$ChosenNumber", &with_set_chosen, false), None);
    assert_eq!(pump_amount("-X", &with_set_chosen, false), None);

    // `Sacrificed$Amount` alongside `has_x = true` + `X:Count$xPaid` (CostPart::SacrificeX)
    // must NOT emit `ThisWay::SacrificedCount` (which only counts `Cause::Effect` sacrifices).
    let sac_x_cost = svars_of(&[
        ("X", "Count$xPaid"),
        ("Y", "Sacrificed$Amount"),
        ("Z", "SVar$Y/Times.2"),
    ]);
    assert_eq!(amount("Y", &sac_x_cost, true), None);
    assert_eq!(amount("Z", &sac_x_cost, true), None);
    assert_eq!(pump_amount("+Z", &sac_x_cost, true), None);

    // Multi-hop `SVar$` chain for resolution `Sacrificed$Amount` (`has_x = false`).
    let sac_res = svars_of(&[("Y", "Sacrificed$Amount"), ("Z", "SVar$Y/Times.2")]);
    assert_eq!(
        amount("Z", &sac_res, false).as_deref(),
        Some("Amount::Times { base: &Amount::ThisWay(ThisWay::SacrificedCount), factor: 2 }")
    );
}

#[test]
fn wave26b_power_up_once_per_permanent_and_own_mana_cost_reduction() {
    // Aerial Doombot (real Paper Power-up creature).
    let doombot = read(
        "Name:Aerial Doombot\nManaCost:2 U\nTypes:Artifact Creature Robot Villain\nPT:2/1\n\
         K:Flying\n\
         A:AB$ PutCounter | Cost$ 5 U | Defined$ Self | CounterType$ P1P1 | CounterNum$ 3 | PowerUp$ True",
    );
    assert_eq!(
        doombot.abilities,
        ["activated!(cost!(\"{5}{U}\"), \
             &[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(3) }], \
             limit = ActivationLimit::OncePerPermanent, \
             cost_reduction = Some(CostReduction::OwnManaCostIfEnteredThisTurn))"]
    );

    // Extremis Elite (Power-up with SubAbility chain).
    let extremis = read(
        "Name:Extremis Elite\nManaCost:2 R\nTypes:Creature Human Villain\nPT:2/2\n\
         A:AB$ PutCounter | Cost$ 4 R | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2 | PowerUp$ True | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1",
    );
    assert!(
        extremis.abilities[0].contains("limit = ActivationLimit::OncePerPermanent")
            && extremis.abilities[0]
                .contains("cost_reduction = Some(CostReduction::OwnManaCostIfEnteredThisTurn)")
            && extremis.abilities[0].contains("target = Some(TargetSpec::AnyTarget)"),
        "{:?}",
        extremis.abilities
    );

    // Stature, Size Shifter (`Cost$ X U U` + `PowerUp$ True`).
    let stature = read(
        "Name:Stature, Size Shifter\nManaCost:U\nTypes:Legendary Creature Human Hero\nPT:1/1\n\
         A:AB$ PutCounter | Cost$ X U U | Defined$ Self | CounterType$ P1P1 | CounterNum$ X | PowerUp$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        stature.abilities[0].contains("cost!(\"{X}{U}{U}\")")
            && stature.abilities[0].contains("amount: Amount::X")
            && stature.abilities[0].contains("limit = ActivationLimit::OncePerPermanent")
            && stature.abilities[0]
                .contains("cost_reduction = Some(CostReduction::OwnManaCostIfEnteredThisTurn)"),
        "{:?}",
        stature.abilities
    );

    // She-Hulk, Jade Defender (up to one target Destroy + PutCounter + PowerUp).
    let she_hulk = read(
        "Name:She-Hulk, Jade Defender\nManaCost:3 G\nTypes:Legendary Creature Hero\nPT:4/4\n\
         K:Reach\nK:Trample\n\
         A:AB$ Destroy | Cost$ 4 G G | ValidTgts$ Artifact,Enchantment | TargetMin$ 0 | TargetMax$ 1 | PowerUp$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        she_hulk.abilities[0].contains("TargetReq::up_to_one(")
            && she_hulk.abilities[0].contains("limit = ActivationLimit::OncePerPermanent")
            && she_hulk.abilities[0]
                .contains("cost_reduction = Some(CostReduction::OwnManaCostIfEnteredThisTurn)"),
        "{:?}",
        she_hulk.abilities
    );

    // Modal activated Power-up ability (`AB$ Charm | PowerUp$ True`).
    let modal_power_up = read(
        "Name:Modal Hero\nManaCost:1 W\nTypes:Creature Human Hero\nPT:2/2\n\
         A:AB$ Charm | Cost$ 4 W | Choices$ M1,M2 | PowerUp$ True | AdditionalDescription$ . Activate only once.\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 3",
    );
    assert!(
        modal_power_up.abilities[0].contains("modal_activated!(")
            && modal_power_up.abilities[0].contains("limit = ActivationLimit::OncePerPermanent")
            && modal_power_up.abilities[0]
                .contains("cost_reduction = Some(CostReduction::OwnManaCostIfEnteredThisTurn)"),
        "{:?}",
        modal_power_up.abilities
    );
}

#[test]
fn wave26b_activated_wave14_cost_reductions_and_per_turn_limits() {
    // Wave 14 `Amount` inside `AB$ ... | ReduceCost$ X` and `ActivationLimit$ X`.
    let party_reduce = read(
        "Name:Party Tactician\nManaCost:2 W\nTypes:Creature Human Warrior\nPT:2/2\n\
         A:AB$ Draw | Cost$ 5 T | NumCards$ 1 | ReduceCost$ X\n\
         SVar:X:Count$Party",
    );
    assert!(
        party_reduce.abilities[0].contains(
            "cost_reduction = Some(CostReduction::PerCount { amount: Amount::PartySize(PlayerRel::You), each: 1 })"
        ),
        "{:?}",
        party_reduce.abilities
    );

    let party_limit = read(
        "Name:Party Duelist\nManaCost:1 R\nTypes:Creature Human Rogue\nPT:2/1\n\
         A:AB$ Pump | Cost$ 1 | Defined$ Self | NumAtt$ +1 | ActivationLimit$ X\n\
         SVar:X:Count$Party",
    );
    assert!(
        party_limit.abilities[0]
            .contains("limit = ActivationLimit::PerTurnAmount(Amount::PartySize(PlayerRel::You))"),
        "{:?}",
        party_limit.abilities
    );

    let greatest_reduce = read(
        "Name:Colossal Channeler\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 6 T | NumCards$ 1 | ReduceCost$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$GreatestCardPower",
    );
    assert!(
        greatest_reduce.abilities[0].contains(
            "cost_reduction = Some(CostReduction::PerCount { amount: Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE), each: 1 })"
        ),
        "{:?}",
        greatest_reduce.abilities
    );

    // Pack Tactics (`Count$Valid Creature.attacking$CardPower`) and Main2 drawn-count trigger.
    let pack_tactics = read(
        "Name:Gnoll Hunter\nManaCost:1 G\nTypes:Creature Gnoll Ranger\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | CheckSVar$ PackTactics | SVarCompare$ GE6 | NoResolvingCheck$ True | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1\n\
         SVar:PackTactics:Count$Valid Creature.attacking$CardPower",
    );
    assert!(
        pack_tactics.abilities[0].contains("Amount::TotalPowerAmong(&Filter::ATTACKING_CREATURE)"),
        "{:?}",
        pack_tactics.abilities
    );

    let main2_drawn = read(
        "Name:Second Main Study\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main2 | ValidPlayer$ You | CheckSVar$ X | SVarCompare$ GE2 | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw\n\
         SVar:X:Count$YouDrewThisTurn",
    );
    assert!(
        main2_drawn.abilities[0].contains("Amount::CardsDrawnThisTurn(PlayerRel::You)"),
        "{:?}",
        main2_drawn.abilities
    );
}

#[test]
fn wave26b_strict_refusals_for_power_up_and_non_paper_scripts() {
    for bad in [
        // `PowerUp$` on a spell line.
        "Name:BadSpellPowerUp\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | PowerUp$ True",
        // `PowerUp$` with non-True value.
        "Name:BadFalsePowerUp\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 R | NumCards$ 1 | PowerUp$ False",
        // `PowerUp$` combined with another activation limit.
        "Name:BadDualLimit1\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 R | NumCards$ 1 | PowerUp$ True | ActivationLimit$ 1",
        "Name:BadDualLimit2\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 R | NumCards$ 1 | PowerUp$ True | Exhaust$ True",
        "Name:BadDualLimit3\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 R | NumCards$ 1 | PowerUp$ True | GameActivationLimit$ 1",
        "Name:BadDualLimit4\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 R | NumCards$ 1 | PowerUp$ True | Boast$ True",
        // `PowerUp$` combined with `ReduceCost$`.
        "Name:BadPowerUpReduce\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 5 R | NumCards$ 1 | PowerUp$ True | ReduceCost$ X\n\
         SVar:X:Count$Party",
        // `PowerUp$` outside the battlefield.
        "Name:BadPowerUpGrave\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 B | NumCards$ 1 | ActivationZone$ Graveyard | PowerUp$ True",
        "Name:BadPowerUpHand\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 4 U | NumCards$ 1 | ActivationZone$ Hand | PowerUp$ True",
        // `PowerUp$` on a loyalty ability.
        "Name:BadPowerUpLoyalty\nManaCost:3 R\nTypes:Planeswalker\nLoyalty:3\n\
         A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | PowerUp$ True | NumCards$ 1",
        // Pure Paper-MTG guard: Alchemy / Un-Set scripts remain strictly refused.
        "Name:BadAlchemyConjure\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ MakeCard | Cost$ 4 U | Name$ Island | Zone$ Hand | PowerUp$ True",
        "Name:BadUnSetContraption\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ AssembleContraption | Amount$ X\nSVar:X:Count$Party",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}
