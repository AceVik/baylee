//! Wave 17c `scriptgen` tests:
//! 1. `S: Mode$ ReduceCost` & `S: Mode$ ForetellAnyTurn` (*Demilich*, *Khalni Hydra*,
//!    *Cosmos Charger*, *Ranar the Ever-Watchful*, *Inquisitive Glimmer*, *Fervent Champion*,
//!    *Strong Back*, *Hamlet Glutton*, *Ice Out*, *Battlefield Thaumaturge*).
//! 2. `S: Mode$ RaiseCost` (*Champion of the Clachan* `BeholdExile<1/...>` + linked LTB return,
//!    *Carth the Lion*, *Eidolon of Obstruction*, *Suppression Field*, *Tithe Taker*,
//!    *Dragon's Prey*, *Kopala, Warden of Waves*, *Terror of the Peaks*).
//! 3. `AffectedZone$ Graveyard` static grants & permissions (*Sedris, the Traitor King*,
//!    *Varolz, the Scar-Striped*, *The Necrobloom*, *Brokkos, Apex of Forever*,
//!    *Tenacious Underdog*, *Timeline Culler*, *Yixlid Jailer*, *Eye of Duskmantle*,
//!    *Iroh, Grand Lotus*, *Wire Surgeons*).
//! 4. `T: Mode$ DiscardedAll` & `Card.TriggeredCards` graveyard exile (*Rielle, the Everwise*,
//!    *Veronica, Dissident Scribe*, *Hostile Investigator*, *Tinybones, Bauble Burglar*,
//!    *Toluz, Clever Conductor*).
//! 5. `DB$/AB$/SP$ Pump` & `PumpAll` with `NumAtt$ Double` / `NumDef$ Double` (*Devilish Valet*,
//!    *Junk Jet*, *Overclocked Electromancer*, *God-Eternal Rhonas*, *Unnatural Growth*).

#![allow(clippy::too_many_lines)]

use super::*;

fn w17c_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Dragon".into(),
            "Elf".into(),
            "Elemental".into(),
            "Giant".into(),
            "Goblin".into(),
            "Hero".into(),
            "Horse".into(),
            "Human".into(),
            "Hydra".into(),
            "Kithkin".into(),
            "Knight".into(),
            "Merfolk".into(),
            "Mutant".into(),
            "Ninja".into(),
            "Rogue".into(),
            "Skeleton".into(),
            "Sliver".into(),
            "Spirit".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec!["Clue".into(), "Equipment".into(), "Vehicle".into()],
        enchantment: vec!["Aura".into(), "Class".into(), "Room".into()],
        spell: vec!["Lesson".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w17c(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w17c_cats();
    let toks = tokens();
    transcode(&parsed, &cats, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, Some(&toks))
        )
    })
}

fn refused_w17c(text: &str) -> bool {
    let cats = w17c_cats();
    let toks = tokens();
    transcode(&parse(text), &cats, Some(&toks)).is_none()
}

#[test]
fn wave17c_reduce_cost_and_foretell_any_turn() {
    // Demilich shape: colored per-count reduction (`{U}` per instant/sorcery cast this turn).
    let demilich = read_w17c(
        "Name:Demilich\nManaCost:U U U U\nTypes:Creature Skeleton Wizard\nPT:4/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | Color$ U | EffectZone$ All\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
    );
    assert!(
        demilich.abilities[0].contains("Modifier::SpellsCostLessManaByAmount")
            && demilich.abilities[0].contains("unit: mana!(\"{U}\")")
            && demilich.abilities[0].contains("colored_only: false"),
        "unexpected Demilich static: {:?}",
        demilich.abilities
    );

    // Khalni Hydra shape with `IgnoreGeneric$ True` (`colored_only: true`).
    let hydra = read_w17c(
        "Name:Khalni Hydra\nManaCost:G G G G G G G G\nTypes:Creature Hydra\nPT:8/8\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ X | Color$ G | IgnoreGeneric$ True | EffectZone$ All\n\
         SVar:X:Count$Valid Creature.Green+YouCtrl",
    );
    assert!(
        hydra.abilities[0].contains("Modifier::SpellsCostLessManaByAmount")
            && hydra.abilities[0].contains("unit: mana!(\"{G}\")")
            && hydra.abilities[0].contains("colored_only: true"),
        "unexpected Khalni Hydra static: {:?}",
        hydra.abilities
    );

    // Cosmos Charger: foretell action cost reduction + foretell on any player's turn.
    let charger = read_w17c(
        "Name:Cosmos Charger\nManaCost:3 U\nTypes:Creature Horse Spirit\nPT:3/3\n\
         K:Flash\nK:Flying\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Static.Foretelling | Activator$ You | Amount$ 1\n\
         S:Mode$ ForetellAnyTurn | ValidPlayer$ You\n\
         K:Foretell:2 U",
    );
    assert!(charger.abilities.iter().any(|a| {
        a.contains(
            "Modifier::ForetellActionCostsLess { who: PlayerRel::You, amount: 1, first_each_turn_only: false }",
        )
    }));
    assert!(
        charger
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::MayForetellAnyTurn { who: PlayerRel::You }"))
    );

    // Ranar cost half: first foretell each turn costs {2} less (`{0}`).
    let ranar = read_w17c(
        "Name:Ranar Half\nManaCost:2 W U\nTypes:Legendary Creature Spirit Warrior\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Foretell | FirstForetell$ True | Activator$ You | Amount$ 2",
    );
    assert!(ranar.abilities[0].contains(
        "Modifier::ForetellActionCostsLess { who: PlayerRel::You, amount: 2, first_each_turn_only: true }"
    ));

    // Inquisitive Glimmer: Room unlock special action costs {1} less.
    let glimmer = read_w17c(
        "Name:Inquisitive Glimmer\nManaCost:W U\nTypes:Enchantment Creature Spirit\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Static.Unlock | Activator$ You | Amount$ 1",
    );
    assert!(
        glimmer.abilities[0]
            .contains("Modifier::UnlockCostsLess { who: PlayerRel::You, amount: 1 }")
    );

    // Fervent Champion & Strong Back: targeted Equip cost reductions.
    let champion = read_w17c(
        "Name:Fervent Champion\nManaCost:R\nTypes:Creature Human Knight\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 3",
    );
    assert!(champion.abilities[0].contains(
        "Modifier::EquipCostsLessIfTargets { who: PlayerRel::You, target_filter: &Filter::This, amount: 3 }"
    ));

    let strong_back = read_w17c(
        "Name:Strong Back\nManaCost:2 G\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         S:Mode$ ReduceCost | ValidCard$ Equipment | ValidTarget$ Enchanted | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 3",
    );
    assert!(
        strong_back.statics.contains("Filter::AttachedToBySource")
            || strong_back
                .abilities
                .iter()
                .any(|a| a.contains("Filter::AttachedToBySource"))
    );
    assert!(strong_back.abilities.iter().any(|a| {
        a.contains("Modifier::EquipCostsLessIfTargets { who: PlayerRel::You")
            && a.contains("amount: 3 }")
    }));

    // Hamlet Glutton: Bargain spell cost reduction.
    let glutton = read_w17c(
        "Name:Hamlet Glutton\nManaCost:5 G G\nTypes:Creature Giant\nPT:6/6\n\
         K:Bargain\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | ValidSpell$ Spell.Bargain | Amount$ 2 | EffectZone$ All",
    );
    assert!(glutton.abilities.iter().any(|a| {
        a.contains("Modifier::AdjustSpellCostByCast")
            && a.contains("delta_per_unit: -2")
            && a.contains("metric: SpellCastMetric::Bargained")
    }));

    // Battlefield Thaumaturge: DistinctTargetsMatching on Instant.YouCtrl,Sorcery.YouCtrl.
    let thaumaturge = read_w17c(
        "Name:Battlefield Thaumaturge\nManaCost:1 U\nTypes:Creature Human Wizard\nPT:2/1\n\
         S:Mode$ ReduceCost | ValidCard$ Instant.YouCtrl,Sorcery.YouCtrl | Type$ Spell | Amount$ X | EffectZone$ All | Relative$ True\n\
         SVar:X:TargetedObjectsDistinct$Valid Creature",
    );
    assert!(
        thaumaturge.abilities[0].contains("Modifier::AdjustSpellCostByCast")
            && thaumaturge.abilities[0].contains("who: PlayerRel::You")
            && thaumaturge.abilities[0].contains("SpellCastMetric::DistinctTargetsMatching"),
        "unexpected Thaumaturge static: {:?}",
        thaumaturge.abilities
    );

    // Honest refusals: Night condition, Ninjutsu cost reduction, MinMana$ 1 on Equip.
    assert!(refused_w17c(
        "Name:Moonragers Slash\nManaCost:2 R\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | Condition$ Night\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3"
    ));
    assert!(refused_w17c(
        "Name:SilverFur\nManaCost:U B\nTypes:Creature Ninja\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Ninjutsu | Activator$ You | Amount$ 1"
    ));
    assert!(refused_w17c(
        "Name:BadEquipFloor\nManaCost:R\nTypes:Creature\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidTarget$ Card.Self | ValidSpell$ Activated.Equip | Activator$ You | Amount$ 3 | MinMana$ 1"
    ));
}

#[test]
fn wave17c_raise_cost_and_behold_exile() {
    // Champion of the Clachan: mandatory BeholdExile<1/Kithkin> + LTB ReturnLinkedExile.
    let clachan = read_w17c(
        "Name:Champion of the Clachan\nManaCost:3 W\nTypes:Creature Kithkin Knight\nPT:4/5\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Cost$ BeholdExile<1/Kithkin> | EffectZone$ All\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Exile | Destination$ Hand | Defined$ ExiledWith",
    );
    assert_eq!(clachan.mandatory_additional_costs.len(), 1);
    assert!(clachan.mandatory_additional_costs[0].starts_with("CostPart::BeholdAndExile("));
    assert!(
        clachan
            .abilities
            .iter()
            .any(|a| a.contains("Effect::ReturnLinkedExile { to: ZoneRef::Hand }")),
        "expected ReturnLinkedExile on LTB trigger: {:?}",
        clachan.abilities
    );

    // Carth the Lion: loyalty counter cost adjustment (`[+1]` and `[-1]`).
    let carth = read_w17c(
        "Name:Carth the Lion\nManaCost:2 B G\nTypes:Legendary Creature Human Warrior\nPT:3/5\n\
         S:Mode$ RaiseCost | ValidCard$ Planeswalker.YouCtrl | Type$ Ability | ValidSpell$ Activated.Loyalty | Activator$ You | Cost$ AddCounter<1/LOYALTY>",
    );
    assert!(
        carth.abilities[0].contains("Modifier::AdjustLoyaltyCost")
            && carth.abilities[0].contains("delta: 1"),
        "unexpected Carth static: {:?}",
        carth.abilities
    );

    let carth_minus = read_w17c(
        "Name:AntiCarth\nManaCost:2 B\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | ValidCard$ Planeswalker.OppCtrl | Type$ Ability | ValidSpell$ Activated.Loyalty | Activator$ Opponent | Cost$ SubCounter<1/LOYALTY>",
    );
    assert!(
        carth_minus.abilities[0].contains("Modifier::AdjustLoyaltyCost")
            && carth_minus.abilities[0].contains("delta: -1")
    );

    // Eidolon of Obstruction: loyalty mana tax.
    let eidolon = read_w17c(
        "Name:Eidolon of Obstruction\nManaCost:1 W\nTypes:Enchantment Creature Spirit\nPT:2/1\n\
         K:First Strike\n\
         S:Mode$ RaiseCost | ValidCard$ Planeswalker.OppCtrl | Type$ Ability | ValidSpell$ Activated.Loyalty | Activator$ Opponent | Amount$ 1",
    );
    assert!(
        eidolon.abilities.iter().any(|a| {
            a.contains("Modifier::LoyaltyAbilitiesCostMore") && a.contains("amount: 1")
        })
    );

    // Suppression Field & Tithe Taker: non-mana activated ability taxes.
    let suppression = read_w17c(
        "Name:Suppression Field\nManaCost:1 W\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | ValidSpell$ Activated.!ManaAbility | Amount$ 2",
    );
    assert!(suppression.abilities[0].contains(
        "Modifier::NonManaAbilitiesCostMore { who: PlayerRel::EachPlayer, filter: &Filter::Any, amount: 2 }"
    ));

    let tithe_taker = read_w17c(
        "Name:Tithe Taker\nManaCost:1 W\nTypes:Creature Human\nPT:2/1\n\
         S:Mode$ RaiseCost | ValidSpell$ Spell,Activated.!ManaAbility | Activator$ Opponent | Amount$ 1 | Condition$ PlayerTurn",
    );
    assert!(
        tithe_taker
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::SpellsCostMore(1)"))
    );
    assert!(
        tithe_taker
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::NonManaAbilitiesCostMore"))
    );

    // Dragon's Prey, Kopala, and Terror of the Peaks: target-conditional taxes.
    let dragons_prey = read_w17c(
        "Name:Dragons Prey\nManaCost:2 B\nTypes:Instant\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | ValidTarget$ Dragon | EffectZone$ All\n\
         A:SP$ Destroy | ValidTgts$ Creature",
    );
    assert!(
        dragons_prey.abilities[0].contains("Modifier::RaiseCostIfTargets")
            && dragons_prey.abilities[0].contains("extra_mana: mana!(\"{2}\")")
            && dragons_prey.abilities[0].contains("abilities: false")
    );

    let kopala = read_w17c(
        "Name:Kopala\nManaCost:1 U U\nTypes:Legendary Creature Merfolk Wizard\nPT:2/2\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Spell | Amount$ 2 | ValidTarget$ Merfolk.YouCtrl\n\
         S:Mode$ RaiseCost | Activator$ Opponent | Type$ Ability | Amount$ 2 | ValidTarget$ Merfolk.YouCtrl",
    );
    assert_eq!(kopala.abilities.len(), 2);
    assert!(kopala.abilities[0].contains("Modifier::SpellsCostMore(2)"));
    assert!(
        kopala.abilities[1].contains("Modifier::RaiseCostIfTargets")
            && kopala.abilities[1].contains("abilities: true")
    );

    let terror = read_w17c(
        "Name:Terror Tax\nManaCost:3 R R\nTypes:Creature Dragon\nPT:5/4\n\
         S:Mode$ RaiseCost | ValidTarget$ Card.Self | Activator$ Opponent | Type$ Spell | Cost$ PayLife<3>",
    );
    assert!(
        terror.abilities[0].contains("Modifier::RaiseCostIfTargets")
            && terror.abilities[0].contains("extra_mana: ManaCost::ZERO")
            && terror.abilities[0].contains("extra_life: 3")
    );

    // Honest refusals: BeholdExile<2/...>, player target on RaiseCostIfTargets.
    assert!(refused_w17c(
        "Name:DoubleBehold\nManaCost:2 W\nTypes:Creature Kithkin\nPT:2/2\n\
         S:Mode$ RaiseCost | ValidCard$ Card.Self | Type$ Spell | Cost$ BeholdExile<2/Kithkin> | EffectZone$ All"
    ));
    assert!(refused_w17c(
        "Name:BadPlayerRaise\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         S:Mode$ RaiseCost | ValidTarget$ Player.Opponent | Activator$ Opponent | Type$ Spell | Amount$ 2"
    ));
}

#[test]
fn wave17c_graveyard_grants_and_permissions() {
    let sedris = read_w17c(
        "Name:Sedris\nManaCost:3 U B R\nTypes:Legendary Creature Zombie Warrior\nPT:5/5\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Unearth:2 B",
    );
    assert!(sedris.abilities[0].contains("Modifier::GrantsUnearth(mana!(\"{2}{B}\"))"));

    let varolz = read_w17c(
        "Name:Varolz\nManaCost:1 B G\nTypes:Legendary Creature Zombie Warrior\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Creature.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Scavenge:CardManaCost",
    );
    assert!(varolz.abilities[0].contains("Modifier::GrantsScavengeManaCost"));

    let necrobloom = read_w17c(
        "Name:The Necrobloom\nManaCost:1 W B G\nTypes:Legendary Creature\nPT:2/7\n\
         S:Mode$ Continuous | Affected$ Land.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Dredge:2",
    );
    assert!(necrobloom.abilities[0].contains("Modifier::GrantsDredge(2)"));

    let brokkos = read_w17c(
        "Name:Brokkos\nManaCost:2 B G U\nTypes:Legendary Creature Elemental\nPT:6/6\n\
         K:Mutate:3 U B G\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True | ValidSA$ Spell.Mutate",
    );
    assert!(brokkos.abilities.iter().any(|a| {
        a.contains("Modifier::MayCastFromGraveyardUsingAlt(GraveyardAltKind::Mutate)")
    }));

    let underdog = read_w17c(
        "Name:Tenacious Underdog\nManaCost:1 B\nTypes:Creature Human Warrior\nPT:3/2\n\
         K:Blitz:2 B B PayLife<2>\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True | ValidSA$ Spell.Blitz",
    );
    assert!(underdog.abilities.iter().any(|a| {
        a.contains("Modifier::MayCastFromGraveyardUsingAlt(GraveyardAltKind::Blitz)")
    }));

    let culler = read_w17c(
        "Name:Timeline Culler\nManaCost:B B\nTypes:Creature\nPT:2/2\n\
         K:Warp:B PayLife<2>\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True | ValidSA$ Spell.Warp",
    );
    assert!(
        culler.abilities.iter().any(|a| {
            a.contains("Modifier::MayCastFromGraveyardUsingAlt(GraveyardAltKind::Warp)")
        })
    );

    let jailer = read_w17c(
        "Name:Yixlid Jailer\nManaCost:1 B\nTypes:Creature Zombie Wizard\nPT:2/1\n\
         S:Mode$ Continuous | Affected$ Card | AffectedZone$ Graveyard | RemoveAllAbilities$ True",
    );
    assert!(
        jailer.abilities[0].contains("Filter::InZone(ZoneRef::Graveyard)")
            && jailer.abilities[0].contains("Modifier::LoseAllAbilities")
    );

    let eye = read_w17c(
        "Name:Eye of Duskmantle\nManaCost:5 U B\nTypes:Creature\nPT:3/8\n\
         S:Mode$ Continuous | Affected$ Card.surveilledThisTurn | AffectedZone$ Graveyard | MayPlay$ True | MayPlayAltManaCost$ PayLife<ConvertedManaCost>",
    );
    assert!(
        eye.abilities
            .iter()
            .any(|a| a.contains("Filter::SurveilledByYouThisTurn"))
            || eye.statics.contains("Filter::SurveilledByYouThisTurn")
    );
    assert!(
        eye.abilities.iter().any(|a| {
            a.contains("Modifier::MayCastWithAltCost")
                && a.contains("grant_zone_permission: true")
                && a.contains("allow_lands: true")
                && a.contains("AltCastPrice::PayLifeEqualToManaValue")
        }),
        "unexpected Eye of Duskmantle abilities: {:?}",
        eye.abilities
    );

    let iroh = read_w17c(
        "Name:Iroh Half\nManaCost:3 U R G\nTypes:Legendary Creature Human\nPT:4/4\n\
         S:Mode$ Continuous | Affected$ Instant.YouOwn,Sorcery.YouOwn | AffectedZone$ Graveyard | AddKeyword$ Flashback:1",
    );
    assert!(iroh.abilities[0].contains("Modifier::GrantsFlashbackCost(mana!(\"{1}\"))"));

    let surgeons = read_w17c(
        "Name:Wire Surgeons\nManaCost:4 B B\nTypes:Creature Human Wizard\nPT:6/5\n\
         S:Mode$ Continuous | Affected$ Artifact.Creature+YouOwn | AffectedZone$ Graveyard | AddKeyword$ Encore:CardManaCost",
    );
    assert!(surgeons.abilities[0].contains("Modifier::GrantsEncoreManaCost"));

    // Honest refusals: ReplaceGraveyard$ Exile on MayPlayAltManaCost, land with ValidSA$ Spell.Warp.
    assert!(refused_w17c(
        "Name:Maestros Ascendancy\nManaCost:U B R\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Instant.YouOwn,Sorcery.YouOwn | AffectedZone$ Graveyard | MayPlay$ True | MayPlayAltManaCost$ CardManaCost | RaiseCost$ Sac<1/Creature> | ReplaceGraveyard$ Exile"
    ));
    assert!(refused_w17c(
        "Name:BadWarpLand\nTypes:Land\n\
         S:Mode$ Continuous | Affected$ Card.Self | AffectedZone$ Graveyard | EffectZone$ Graveyard | MayPlay$ True | ValidSA$ Spell.Warp"
    ));
}

#[test]
fn wave17c_discarded_all_triggers() {
    // Rielle: first discard each turn + TriggerCount$Amount.
    let rielle = read_w17c(
        "Name:Rielle\nManaCost:1 U R\nTypes:Legendary Creature Human Wizard\nPT:0/3\n\
         T:Mode$ DiscardedAll | ValidPlayer$ You | TriggerZones$ Battlefield | FirstTime$ True | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:TriggerCount$Amount",
    );
    assert!(rielle.abilities[0].contains(
        "Trigger::OneOrMoreCardsDiscarded { who: PlayerRel::You, filter: &Filter::Any, first_this_turn: true, per_player: true }"
    ));
    assert!(rielle.abilities[0].contains("Amount::EventAmount"));

    // Veronica: nonland filter + FirstTime$ True.
    let veronica = read_w17c(
        "Name:Veronica\nManaCost:2 R\nTypes:Legendary Creature Human Rogue\nPT:3/3\n\
         T:Mode$ DiscardedAll | ValidPlayer$ You | ValidCard$ Card.nonLand | TriggerZones$ Battlefield | FirstTime$ True | Execute$ TrigClue\n\
         SVar:TrigClue:DB$ Investigate",
    );
    assert!(veronica.abilities[0].contains("Trigger::OneOrMoreCardsDiscarded"));
    assert!(veronica.abilities[0].contains("first_this_turn: true"));

    // Hostile Investigator: "Whenever one or more players discard..." -> per_player: false + once_per_turn.
    let investigator = read_w17c(
        "Name:Hostile Investigator\nManaCost:3 B\nTypes:Creature\nPT:4/3\n\
         T:Mode$ DiscardedAll | ValidPlayer$ Player | TriggerZones$ Battlefield | ActivationLimit$ 1 | Execute$ TrigClue\n\
         SVar:TrigClue:DB$ Investigate\n\
         Oracle:Whenever one or more players discard one or more cards, investigate. This ability triggers only once each turn.",
    );
    assert!(investigator.abilities[0].contains("per_player: false"));
    assert!(investigator.abilities[0].contains("once_per_turn = true"));

    // Tinybones: "Whenever an opponent discards..." -> per_player: true.
    let tinybones = read_w17c(
        "Name:Tinybones\nManaCost:1 B\nTypes:Legendary Creature Skeleton Rogue\nPT:1/3\n\
         T:Mode$ DiscardedAll | ValidPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1\n\
         Oracle:Whenever an opponent discards one or more cards, draw a card.",
    );
    assert!(tinybones.abilities[0].contains("who: PlayerRel::EachOpponent"));
    assert!(tinybones.abilities[0].contains("per_player: true"));

    // Toluz: exile triggered discarded cards from graveyard linked to source + dies return to hand.
    let toluz = read_w17c(
        "Name:Toluz\nManaCost:W U B\nTypes:Legendary Creature Human Rogue\nPT:3/1\n\
         T:Mode$ DiscardedAll | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ChangeType$ Card.TriggeredCards\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Exile | Destination$ Hand | Defined$ ExiledWith",
    );
    assert!(toluz.abilities[0].contains("zone: ChooseZone::Found"));
    assert!(toluz.abilities[0].contains("not_chosen_movement: ChooseMove::ExileLinked"));
    assert!(toluz.abilities[1].contains("Effect::ReturnLinkedExile { to: ZoneRef::Hand }"));

    // Honest refusals: Random$ True, missing TriggerZones$, Card.Self.
    assert!(refused_w17c(
        "Name:BadRandom\nManaCost:1 R\nTypes:Enchantment\n\
         T:Mode$ DiscardedAll | ValidPlayer$ You | Random$ True | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
    assert!(refused_w17c(
        "Name:BadUnzoned\nManaCost:1 R\nTypes:Enchantment\n\
         T:Mode$ DiscardedAll | ValidPlayer$ You | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
    assert!(refused_w17c(
        "Name:BadSelfDiscardAll\nManaCost:1 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DiscardedAll | ValidCard$ Card.Self | TriggerZones$ Graveyard | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1"
    ));
}

#[test]
fn wave17c_pump_double_effects() {
    // Devilish Valet: Defined$ Self -> DoublePtSubject::Source.
    let valet = read_w17c(
        "Name:Devilish Valet\nManaCost:2 R\nTypes:Creature\nPT:1/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.Other+YouCtrl | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ Double",
    );
    assert!(valet.abilities[0].contains(
        "Effect::DoublePT { subject: DoublePtSubject::Source, double_power: true, double_toughness: false"
    ));

    // Junk Jet: Defined$ Equipped -> DoublePtSubject::Attached.
    let junk_jet = read_w17c(
        "Name:Junk Jet\nManaCost:1 R\nTypes:Artifact Equipment\n\
         A:AB$ Pump | Cost$ Sac<1/Artifact> | Defined$ Equipped | NumAtt$ Double\n\
         K:Equip:2",
    );
    assert!(
        junk_jet
            .abilities
            .iter()
            .any(|a| a.contains("subject: DoublePtSubject::Attached, double_power: true"))
    );

    // Overclocked Electromancer: TriggeredAttackerLKICopy -> DoublePtSubject::EventObject.
    let electromancer = read_w17c(
        "Name:Electromancer\nManaCost:2 R\nTypes:Creature Wizard\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Creature.YouCtrl | Alone$ True | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttackerLKICopy | NumAtt$ Double",
    );
    assert!(electromancer.abilities[0].contains("subject: DoublePtSubject::EventObject"));

    // God-Eternal Rhonas: PumpAll with Vigilance.
    let rhonas = read_w17c(
        "Name:God-Eternal Rhonas\nManaCost:3 G G\nTypes:Legendary Creature Zombie\nPT:5/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ PumpAll | ValidCards$ Creature.Other+YouCtrl | NumAtt$ Double | KW$ Vigilance",
    );
    assert!(rhonas.abilities[0].contains("subject: DoublePtSubject::Filter("));
    assert!(rhonas.abilities[0].contains("double_power: true, double_toughness: false"));
    assert!(rhonas.abilities[0].contains("KeywordSet::VIGILANCE"));

    // Unnatural Growth: PumpAll doubling both power and toughness.
    let unnatural = read_w17c(
        "Name:Unnatural Growth\nManaCost:1 G G G G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ Double | NumDef$ Double",
    );
    assert!(unnatural.abilities[0].contains("double_power: true, double_toughness: true"));

    // Multi-target spell: Choose up to two target creatures -> DoublePtSubject::Targets.
    let multi = read_w17c(
        "Name:Twin Surge\nManaCost:3 G G\nTypes:Sorcery\n\
         A:SP$ Pump | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature | NumAtt$ Double | NumDef$ Double\n\
         Oracle:Choose up to two target creatures. Double their power and toughness until end of turn.",
    );
    assert!(multi.abilities.iter().any(|e| e.contains(
        "subject: DoublePtSubject::Targets, double_power: true, double_toughness: true"
    )));

    // Honest refusals: Perpetual duration, mixed +2 and Double.
    assert!(refused_w17c(
        "Name:BadPerpetualDouble\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ Double | Duration$ Perpetual"
    ));
    assert!(refused_w17c(
        "Name:BadMixedDouble\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ Pump | Defined$ Self | NumAtt$ +2 | NumDef$ Double"
    ));
}
