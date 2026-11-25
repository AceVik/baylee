use super::*;

fn wave26d_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Angel".into(),
            "Demon".into(),
            "Dragon".into(),
            "Eldrazi".into(),
            "Goblin".into(),
            "Human".into(),
            "Imp".into(),
            "Soldier".into(),
            "Vampire".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Equipment".into(),
            "Food".into(),
            "Treasure".into(),
        ],
        enchantment: vec!["Aura".into()],
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

fn read_w26d(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &wave26d_cats(), Some(&tokens())).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &wave26d_cats(), Some(&tokens()))
        )
    })
}

fn refused_w26d(text: &str) -> bool {
    transcode(&parse(text), &wave26d_cats(), Some(&tokens())).is_none()
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave26d_sacrifice_any_number_and_this_way_sacrificed_count() {
    // Angelic Aberration shape: ETB SacrificeAnyNumber -> CreateTokenN(ThisWay::SacrificedCount).
    let aberration = read_w26d(
        "Name:Eldrazi Sacrificer\nManaCost:5 W\nTypes:Creature Eldrazi Angel\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Creature.powerLE1 | RememberSacrificed$ True | Optional$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenAmount$ TokenX | TokenScript$ r_1_1_goblin | TokenOwner$ You | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Creature.powerLE1+YouCtrl\n\
         SVar:TokenX:Remembered$Amount",
    );
    assert!(
        aberration.abilities[0]
            .contains("Effect::SacrificeAnyNumber { who: PlayerRel::You, filter:")
            && aberration.abilities[0]
                .contains("Effect::CreateTokenN { token: &generated_tokens::"),
        "{:?}",
        aberration.abilities
    );
    assert!(
        aberration.abilities[0].contains("Amount::ThisWay(ThisWay::SacrificedCount)"),
        "{:?}",
        aberration.abilities
    );

    // Renounce shape: Sacrifice any number of permanents -> GainLife(2 * SacrificedCount).
    let renounce = read_w26d(
        "Name:Renounce Form\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Permanent | RememberSacrificed$ True | Optional$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ LifeX | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Permanent.YouCtrl\n\
         SVar:LifeX:Remembered$Amount/Twice",
    );
    assert!(
        renounce.abilities[0].contains(
            "Effect::SacrificeAnyNumber { who: PlayerRel::You, filter: &Filter::Any }"
        ) && renounce.abilities[0].contains(
            "Effect::GainLife { amount: Amount::Times { base: &Amount::ThisWay(ThisWay::SacrificedCount), factor: 2 } }"
        ),
        "{:?}",
        renounce.abilities
    );

    // Landslide / Last-Ditch Effort shape: SacrificeAnyNumber -> targeted DealDamage(SacrificedCount).
    let landslide = read_w26d(
        "Name:Mountain Slide\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Mountain | RememberSacrificed$ True | Optional$ True | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ DmgX | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Mountain.YouCtrl\n\
         SVar:DmgX:Remembered$Amount",
    );
    assert!(
        landslide.abilities[0].contains("Effect::SacrificeAnyNumber { who: PlayerRel::You, filter:")
            && landslide.abilities[0].contains(
                "Effect::DealDamage { amount: Amount::ThisWay(ThisWay::SacrificedCount), target: TargetSpec::AnyTarget }"
            ),
        "{:?}",
        landslide.abilities
    );

    // God-Eternal Bontu ETB shape: SacrificeAnyNumber(Permanent.Other) -> DrawCards(SacrificedCount).
    let bontu = read_w26d(
        "Name:Eternal Harvester\nManaCost:3 B B\nTypes:Creature Zombie\nPT:5/6\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Permanent.Other | RememberSacrificed$ True | Optional$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ DrawX | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Permanent.YouCtrl+Other\n\
         SVar:DrawX:Remembered$Amount",
    );
    assert!(
        bontu.abilities[0].contains("Effect::SacrificeAnyNumber")
            && bontu.abilities[0].contains(
                "Effect::DrawCards { amount: Amount::ThisWay(ThisWay::SacrificedCount) }"
            ),
        "{:?}",
        bontu.abilities
    );

    // Scapeshift shape: SacrificeAnyNumber(Land) -> SearchLibraryUpTo(Land, SacrificedCount, BATTLEFIELD_TAPPED).
    let scapeshift = read_w26d(
        "Name:Land Shift\nManaCost:2 G G\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Land | RememberSacrificed$ True | Optional$ True | SubAbility$ DBShift\n\
         SVar:DBShift:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land | ChangeNum$ ShiftX | Tapped$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Land.YouCtrl\n\
         SVar:ShiftX:Remembered$Amount",
    );
    assert!(
        scapeshift.abilities[0].contains("Effect::SacrificeAnyNumber")
            && scapeshift.abilities[0].contains(
                "Effect::SearchLibraryUpTo { filter: &Filter::LAND, count: Amount::ThisWay(ThisWay::SacrificedCount), find: &Find::BATTLEFIELD_TAPPED }"
            ),
        "{:?}",
        scapeshift.abilities
    );

    // War of the Spark chapter II shape: compound Count$Valid /Plus. + opposing SacrificeAmountForPlayers.
    let war_edict = read_w26d(
        "Name:Spark Edict\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Creature,Planeswalker | RememberSacrificed$ True | Optional$ True | SubAbility$ DBEdict\n\
         SVar:DBEdict:DB$ Sacrifice | Defined$ Player.Opponent | SacValid$ Creature,Planeswalker | Amount$ EdictX\n\
         SVar:SacX:Count$Valid Creature.YouCtrl/Plus.SacPlanes\n\
         SVar:SacPlanes:Count$Valid Planeswalker.YouCtrl\n\
         SVar:EdictX:Remembered$Amount",
    );
    assert!(
        war_edict.abilities[0].contains("Effect::SacrificeAnyNumber { who: PlayerRel::You")
            && war_edict.abilities[0]
                .contains("Effect::SacrificeAmountForPlayers { who: PlayerRel::Opponent, filter:")
            && war_edict.abilities[0].contains("amount: Amount::ThisWay(ThisWay::SacrificedCount)"),
        "{:?}",
        war_edict.abilities
    );

    // Voracious Fell Beast / Visions of Ruin shape: each opponent sacrifices -> tokens equal to SacrificedCount.
    let fell_beast = read_w26d(
        "Name:Fell Eater\nManaCost:4 B B\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ Opponent | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBTok\n\
         SVar:DBTok:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$RememberedSize",
    );
    assert!(
        fell_beast.abilities[0].contains(
            "Effect::SacrificeFilter { who: PlayerRel::Opponent, filter: &Filter::CREATURE }"
        ) && fell_beast.abilities[0].contains("amount: Amount::ThisWay(ThisWay::SacrificedCount)"),
        "{:?}",
        fell_beast.abilities
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave26d_remember_sacrificed_lki_stats_and_multi_step_chains() {
    // Tribute to Hunger shape: target opponent sacrifices -> you gain SacrificedTotalToughness.
    let tribute = read_w26d(
        "Name:Hunger Tribute\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Sacrifice | SacValid$ Creature | ValidTgts$ Opponent | Amount$ 1 | RememberSacrificed$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardToughness",
    );
    assert!(
        tribute.abilities[0].contains(
            "Effect::SacrificeFilter { who: PlayerRel::Chosen, filter: &Filter::CREATURE }, \
             Effect::GainLife { amount: Amount::SacrificedTotalToughness }"
        ),
        "{:?}",
        tribute.abilities
    );

    // Devour Flesh shape: target player sacrifices -> that player gains SacrificedTotalToughness.
    let devour = read_w26d(
        "Name:Flesh Devour\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Sacrifice | ValidTgts$ Player | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ Targeted | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardToughness",
    );
    assert!(
        devour.abilities[0].contains(
            "Effect::SacrificeFilter { who: PlayerRel::Chosen, filter: &Filter::CREATURE }, \
             Effect::GainLifeFor { amount: Amount::SacrificedTotalToughness, who: PlayerRel::Chosen }"
        ),
        "{:?}",
        devour.abilities
    );

    // Disciple of Bolas shape: multi-step LKI chain (GainLife + DrawCards).
    let disciple = read_w26d(
        "Name:Bolas Disciple\nManaCost:3 B\nTypes:Creature Human Wizard\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Amount$ 1 | SacValid$ Creature.Other | RememberSacrificed$ True | SubAbility$ GainLife\n\
         SVar:GainLife:DB$ GainLife | LifeAmount$ X | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardPower",
    );
    assert!(
        disciple.abilities[0].contains("Effect::SacrificeFilter { who: PlayerRel::You")
            && disciple.abilities[0]
                .contains("Effect::GainLife { amount: Amount::SacrificedPower }")
            && disciple.abilities[0]
                .contains("Effect::DrawCards { amount: Amount::SacrificedPower }"),
        "{:?}",
        disciple.abilities
    );

    // Rupture shape: sacrifice a creature -> DamageAll (cards + players) equal to power.
    let rupture = read_w26d(
        "Name:Power Rupture\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Amount$ 1 | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ RuptureDmg\n\
         SVar:RuptureDmg:DB$ DamageAll | ValidCards$ Creature.withoutFlying | ValidPlayers$ Player | NumDmg$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardPower",
    );
    assert!(
        rupture.abilities[0].contains("Effect::DealDamageEach { amount: Amount::SacrificedPower")
            && rupture.abilities[0].contains(
                "Effect::DealDamage { amount: Amount::SacrificedPower, target: TargetSpec::Player(PlayerRel::EachPlayer) }"
            ),
        "{:?}",
        rupture.abilities
    );

    // Bound shape: sacrifice a creature -> ChooseCardEx up to SacrificedDistinctColors from GY to hand, then ExileSelf.
    let bound = read_w26d(
        "Name:Color Bound\nManaCost:3 B G\nTypes:Instant\n\
         A:SP$ Sacrifice | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBReturnChoose\n\
         SVar:DBReturnChoose:DB$ ChooseCard | Defined$ You | Choices$ Card.YouOwn | ChoiceZone$ Graveyard | Amount$ X | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ ChosenCard | SubAbility$ ExileSelf\n\
         SVar:ExileSelf:DB$ ChangeZone | Origin$ Stack | Destination$ Exile | Defined$ Self | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardNumColors",
    );
    assert!(
        bound.abilities[0]
            .contains("Effect::SacrificeFilter { who: PlayerRel::You, filter: &Filter::CREATURE }")
            && bound.abilities[0].contains("max: Amount::SacrificedDistinctColors")
            && bound.abilities[0].contains("movement: ChooseMove::Hand")
            && bound.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        bound.abilities
    );

    // Myriad Construct shape: SacrificeAll Defined$ Self | RememberSacrificed$ True -> tokens equal to SacrificedTotalPower.
    let myriad = read_w26d(
        "Name:Splitting Construct\nManaCost:4\nTypes:Artifact Creature\nPT:4/4\n\
         T:Mode$ BecomesTarget | ValidTarget$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ SacrificeAll | Defined$ Self | RememberSacrificed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenAmount$ Y | TokenScript$ r_1_1_goblin | TokenOwner$ You | SubAbility$ DBCleanup\n\
         SVar:Y:RememberedLKI$CardPower\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        myriad.abilities[0].contains("Effect::SacrificeSelfThen { then: &[Effect::CreateTokenN {")
            && myriad.abilities[0].contains("amount: Amount::SacrificedTotalPower"),
        "{:?}",
        myriad.abilities
    );

    // SacrificeAnyNumber + SacrificedTotalManaValue.
    let cmc_sac = read_w26d(
        "Name:Mana Harvest\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Artifact | RememberSacrificed$ True | Optional$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Artifact.YouCtrl\n\
         SVar:X:RememberedLKI$CardManaCost",
    );
    assert!(
        cmc_sac.abilities[0].contains("Effect::SacrificeAnyNumber")
            && cmc_sac.abilities[0]
                .contains("Effect::GainLife { amount: Amount::SacrificedTotalManaValue }"),
        "{:?}",
        cmc_sac.abilities
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave26d_temp_modifiers_abeyance_must_block_and_uncounterable() {
    // Abeyance: target player can't cast instants/sorceries and can't activate non-mana abilities this turn. Draw a card.
    let abeyance = read_w26d(
        "Name:Abeyance Form\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Player | StaticAbilities$ STCantBeCast,STCantBeActivated | RememberObjects$ Targeted | SubAbility$ DBDraw\n\
         SVar:STCantBeCast:Mode$ CantBeCast | ValidCard$ Instant,Sorcery | Caster$ Player.IsRemembered\n\
         SVar:STCantBeActivated:Mode$ CantBeActivated | ValidCard$ Card | ValidSA$ Activated.!ManaAbility | Activator$ Player.IsRemembered\n\
         SVar:DBDraw:DB$ Draw",
    );
    assert!(
        abeyance.abilities[0].contains("Modifier::PlayersCantCast")
            && abeyance.abilities[0].contains(
                "Effect::AddTempModifier { target: TargetSpec::Player(PlayerRel::Chosen), \
                 modifier: Modifier::PlayerCantActivateNonMana { who: PlayerRel::EachPlayer }, \
                 duration: Duration::UntilEndOfTurn }"
            )
            && abeyance.abilities[0].contains("Effect::draw(1)"),
        "{:?}",
        abeyance.abilities
    );

    // Global opponent activation lock until your next turn.
    let global_lock = read_w26d(
        "Name:Quiet Turn\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ STLock\n\
         SVar:STLock:Mode$ CantBeActivated | ValidCard$ Card | ValidSA$ Activated.!ManaAbility | Activator$ Opponent",
    );
    assert!(
        global_lock.abilities[0].contains(
            "Effect::AddGlobalTempModifier { filter: &Filter::Any, \
             modifier: Modifier::PlayerCantActivateNonMana { who: PlayerRel::EachOpponent }, \
             duration: Duration::UntilYourNextTurn }"
        ),
        "{:?}",
        global_lock.abilities
    );

    // Academic Dispute / Provoke shape: targeted MustBlock (Card.IsRemembered).
    let dispute = read_w26d(
        "Name:Forced Block\nManaCost:R\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | ExileOnMoved$ Battlefield | StaticAbilities$ MustBlock\n\
         SVar:MustBlock:Mode$ MustBlock | ValidCreature$ Card.IsRemembered",
    );
    assert!(
        dispute.abilities[0].contains(
            "Effect::AddTempModifier { target: TargetSpec::Object(&Filter::CREATURE), \
             modifier: Modifier::MustBlock, duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        dispute.abilities
    );

    // Predatory Rampage shape: global MustBlock (Creature.OppCtrl).
    let rampage = read_w26d(
        "Name:All Block\nManaCost:3 G G\nTypes:Sorcery\n\
         A:SP$ Effect | StaticAbilities$ STBlock\n\
         SVar:STBlock:Mode$ MustBlock | ValidCreature$ Creature.OppCtrl",
    );
    assert!(
        rampage.abilities[0].contains(
            "Effect::AddGlobalTempModifier { filter: &Filter::OPPONENT_CREATURE, modifier: Modifier::MustBlock, \
             duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        rampage.abilities
    );

    // Determined / Veil of Summer shape: Event$ Counter | ValidSA$ Spell.YouCtrl | Layer$ CantHappen.
    let determined = read_w26d(
        "Name:Uncounterable Turn\nManaCost:G U\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ CantBeCountered | SubAbility$ DBDraw\n\
         SVar:CantBeCountered:Event$ Counter | ValidSA$ Spell.YouCtrl | Layer$ CantHappen\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        determined.abilities[0].contains(
            "Effect::AddGlobalTempModifier { filter: &Filter::Any, \
             modifier: Modifier::SpellsCantBeCountered { who: PlayerRel::You, filter: &Filter::Any }, \
             duration: Duration::UntilEndOfTurn }"
        ) && determined.abilities[0].contains("Effect::draw(1)"),
        "{:?}",
        determined.abilities
    );

    // Creature spells you control can't be countered while source on battlefield (Duration$ LeavesPlay).
    let creature_uncounterable = read_w26d(
        "Name:Ward Totem\nManaCost:2 G\nTypes:Artifact\n\
         A:AB$ Effect | Cost$ T | Duration$ LeavesPlay | ReplacementEffects$ AntiMagic\n\
         SVar:AntiMagic:Event$ Counter | ValidSA$ Spell.Creature+YouCtrl | Layer$ CantHappen",
    );
    assert!(
        creature_uncounterable.abilities[0].contains(
            "Effect::AddGlobalTempModifier { filter: &Filter::Any, \
             modifier: Modifier::SpellsCantBeCountered { who: PlayerRel::You, filter: &Filter::CREATURE }, \
             duration: Duration::WhileSourceOnBattlefield }"
        ),
        "{:?}",
        creature_uncounterable.abilities
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave26d_gain_control_ex_and_gain_control_all_ex() {
    // Broadcast Takeover / Mob Rule / Insurrection shape: GainControlAllEx with Untap + Haste + EOT.
    let broadcast = read_w26d(
        "Name:Artifact Takeover\nManaCost:2 R R R\nTypes:Sorcery\n\
         A:SP$ GainControl | AllValid$ Artifact.OppCtrl | Untap$ True | AddKWs$ Haste | LoseControl$ EOT",
    );
    assert!(
        broadcast.abilities[0].contains(
            "Effect::GainControlAllEx { filter: &GAIN_CONTROL1, to: PlayerRel::You, \
             duration: Duration::UntilEndOfTurn, untap: true, haste_until_eot: true }"
        ),
        "{:?}",
        broadcast.abilities
    );

    // Ashiok ultimate shape: player-targeted GainControlAllEx with Filter::ControlledBy(PlayerRel::Chosen).
    let ashiok = read_w26d(
        "Name:Mind Steal\nManaCost:4 U B\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Opponent | AllValid$ Creature.TargetedPlayerCtrl | NewController$ You",
    );
    assert!(
        ashiok.abilities[0].contains(
            "Effect::GainControlAllEx { filter: &GAIN_CONTROL1, to: PlayerRel::You, \
             duration: Duration::Permanent, untap: false, haste_until_eot: false }"
        ) && ashiok
            .statics
            .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{:?} / {}",
        ashiok.abilities,
        ashiok.statics
    );

    // Charisma shape: DamageDone TriggeredTarget + LoseControl$ LeavesPlay -> GainControlEx(EventObject, WhileSourceOnBattlefield).
    let charisma = read_w26d(
        "Name:Charisma Form\nManaCost:U U U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ DamageDone | ValidSource$ Card.AttachedBy | ValidTarget$ Creature | TriggerZones$ Battlefield | Execute$ Ctrl\n\
         SVar:Ctrl:DB$ GainControl | Defined$ TriggeredTarget | LoseControl$ LeavesPlay",
    );
    assert!(
        charisma.abilities.iter().any(|a| a.contains(
            "Effect::GainControlEx { target: TargetSpec::EventObject, to: PlayerRel::You, \
             duration: Duration::WhileSourceOnBattlefield, untap: false, haste_until_eot: false }"
        )),
        "{:?}",
        charisma.abilities
    );

    // Contested War Zone shape: DamageDone Defined$ Self + NewController$ TriggeredSourceController -> EventSourceController.
    let war_zone = read_w26d(
        "Name:Contested Ground\nTypes:Land\n\
         T:Mode$ DamageDone | ValidSource$ Creature | ValidTarget$ You | CombatDamage$ True | Execute$ TrigControl | TriggerZones$ Battlefield\n\
         SVar:TrigControl:DB$ GainControl | Defined$ Self | NewController$ TriggeredSourceController",
    );
    assert!(
        war_zone.abilities[0].contains(
            "Effect::GainControlEx { target: TargetSpec::ThisObject, to: PlayerRel::EventSourceController, \
             duration: Duration::Permanent, untap: false, haste_until_eot: false }"
        ),
        "{:?}",
        war_zone.abilities
    );

    // Fractured Loyalty shape: BecomesTarget TriggeredTargetLKICopy + TriggeredSourceController -> EventPlayer.
    let loyalty = read_w26d(
        "Name:Fickle Bond\nManaCost:1 R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ BecomesTarget | ValidTarget$ Card.EnchantedBy | TriggerZones$ Battlefield | Execute$ TrigGainControl\n\
         SVar:TrigGainControl:DB$ GainControl | Defined$ TriggeredTargetLKICopy | NewController$ TriggeredSourceController",
    );
    assert!(
        loyalty.abilities.iter().any(|a| a.contains(
            "Effect::GainControlEx { target: TargetSpec::EventObject, to: PlayerRel::EventPlayer, \
             duration: Duration::Permanent, untap: false, haste_until_eot: false }"
        )),
        "{:?}",
        loyalty.abilities
    );

    // Blim shape: targeted permanent you control given to DamagedPlayer.
    let blim = read_w26d(
        "Name:Gift Imp\nManaCost:2 B R\nTypes:Creature Imp\nPT:4/3\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigControl\n\
         SVar:TrigControl:DB$ GainControl | ValidTgts$ Permanent.YouCtrl | NewController$ TriggeredTarget",
    );
    assert!(
        blim.abilities[0].contains(
            "Effect::GainControlEx { target: TargetSpec::Object(&Filter::ControlledByYou), \
             to: PlayerRel::DamagedPlayer, duration: Duration::Permanent, untap: false, haste_until_eot: false }"
        ),
        "{:?}",
        blim.abilities
    );

    // Wellspring shape: Defined$ Enchanted + LoseControl$ EOT.
    let wellspring = read_w26d(
        "Name:Borrow Spring\nManaCost:1 G W\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigGain\n\
         SVar:TrigGain:DB$ GainControl | Defined$ Enchanted | LoseControl$ EOT",
    );
    assert!(
        wellspring.abilities.iter().any(|a| a.contains(
            "Effect::GainControlEx { target: TargetSpec::Object(&Filter::AttachedToBySource), \
             to: PlayerRel::You, duration: Duration::UntilEndOfTurn, untap: false, haste_until_eot: false }"
        )),
        "{:?}",
        wellspring.abilities
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave26d_choose_number_ex_and_move_attachment() {
    // By Invitation Only: SP$ ChooseNumber 0..=13 -> SacrificeAmountForPlayers(ResolutionChosenNumber).
    let invitation = read_w26d(
        "Name:Invitation Sweep\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChooseNumber | Defined$ You | Min$ 0 | Max$ 13 | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | Defined$ Player | SacValid$ Creature | Amount$ X\n\
         SVar:X:Count$ChosenNumber",
    );
    assert!(
        invitation.abilities[0].contains(
            "Effect::ChooseNumberEx { chooser: PlayerRel::You, min: Amount::Fixed(0), \
             max: Amount::Fixed(13), prompt: NumberChoiceKind::Choice, pay_cost: None, then: &[] }"
        ) && invitation.abilities[0].contains(
            "Effect::SacrificeAmountForPlayers { who: PlayerRel::EachPlayer, \
             filter: &Filter::CREATURE, amount: Amount::ResolutionChosenNumber }"
        ),
        "{:?}",
        invitation.abilities
    );

    // Variable energy payment ChooseNumberEx + ResolutionChosenNumber.
    let energy_pay = read_w26d(
        "Name:Energy Blast\nManaCost:R\nTypes:Instant\n\
         A:SP$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ 3 | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseNumber | Max$ Max | ListTitle$ amount of energy to pay | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:Max:Count$YourCountersEnergy\n\
         SVar:X:Count$ChosenNumber",
    );
    assert!(
        energy_pay.abilities[0].contains(
            "Effect::ChooseNumberEx { chooser: PlayerRel::You, min: Amount::Fixed(0), \
             max: Amount::CountersOnYou(CounterKind::Energy), \
             prompt: NumberChoiceKind::EnergyPayment, pay_cost: Some(VariablePayKind::Energy), then: &[Effect::DealDamage"
        ) && energy_pay.abilities[0].contains(
            "Effect::DealDamage { amount: Amount::ResolutionChosenNumber, target: TargetSpec::Object(&Filter::CREATURE) }"
        ),
        "{:?}",
        energy_pay.abilities
    );

    // Variable generic mana payment and life payment ChooseNumberEx.
    let mana_pay = read_w26d(
        "Name:Mana Sink\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChooseNumber | Defined$ You | Min$ 0 | Max$ 5 | ListTitle$ amount of mana to pay | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ChosenNumber",
    );
    assert!(
        mana_pay.abilities[0].contains(
            "prompt: NumberChoiceKind::ManaPayment, pay_cost: Some(VariablePayKind::GenericMana), then: &[Effect::DrawCards"
        ) && mana_pay.abilities[0]
            .contains("Effect::DrawCards { amount: Amount::ResolutionChosenNumber }"),
        "{:?}",
        mana_pay.abilities
    );

    let life_pay = read_w26d(
        "Name:Blood Choice\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChooseNumber | ValidTgts$ Opponent | Min$ 1 | Max$ 5 | ListTitle$ amount of life to pay | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:X:Count$ChosenNumber",
    );
    assert!(
        life_pay.abilities[0].contains(
            "Effect::ChooseNumberEx { chooser: PlayerRel::Chosen, min: Amount::Fixed(1), \
             max: Amount::Fixed(5), prompt: NumberChoiceKind::Choice, \
             pay_cost: Some(VariablePayKind::Life), then: &[Effect::DrawCards { amount: Amount::ResolutionChosenNumber }, Effect::gain_life(2)] }"
        ),
        "{:?}",
        life_pay.abilities
    );

    // Inventory Management: RepeatEach Aura/Equipment you control -> MoveAttachment(optional: true).
    let inventory = read_w26d(
        "Name:Gear Shift\nManaCost:R W\nTypes:Instant\n\
         A:SP$ RepeatEach | RepeatCards$ Aura.YouCtrl,Equipment.YouCtrl | RepeatSubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Choices$ Creature.YouCtrl | ChoiceTitle$ Choose a creature you control | Object$ Remembered | Optional$ True",
    );
    assert!(
        inventory.abilities[0].contains("Effect::RepeatEach")
            && inventory.abilities[0].contains(
                "Effect::MoveAttachment { attachment: TargetSpec::EventObject, \
                 recipient_filter: &Filter::YOUR_CREATURE, must_change_recipient: false, \
                 optional: true, then_if_moved: &[] }"
            ),
        "{:?}",
        inventory.abilities
    );

    // ChooseCard battlefield Aura -> MoveAttachment(must_change_recipient: true) with then_if_moved.
    let move_aura = read_w26d(
        "Name:Aura Shift\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | Choices$ Aura.YouCtrl | ChoiceZone$ Battlefield | Mandatory$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ ChosenCard | Choices$ Creature | Move$ True | RememberAttached$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        move_aura.abilities[0].contains("Effect::ChooseCard")
            && move_aura.abilities[0].contains(
                "Effect::MoveAttachment { attachment: TargetSpec::EventObject, \
                 recipient_filter: &Filter::CREATURE, must_change_recipient: true, \
                 optional: false, then_if_moved: &[Effect::draw(1)] }"
            ),
        "{:?}",
        move_aura.abilities
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave26d_change_zone_remember_changed_change_zone_all_and_copy_permanent() {
    // Through the Breach shape: Hand -> Battlefield + RememberChanged -> Animate(Haste, AtEOT$ Sacrifice).
    let breach = read_w26d(
        "Name:Breach Rush\nManaCost:4 R\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.YouCtrl | SubAbility$ DBPump | RememberChanged$ True | ForgetOtherRemembered$ True\n\
         SVar:DBPump:DB$ Animate | Keywords$ Haste | Defined$ Remembered | Duration$ Permanent | AtEOT$ Sacrifice | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice that creature at the beginning of the next end step.",
    );
    assert!(
        breach.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && breach.abilities[0].contains("movement: ChooseMove::Battlefield { tapped: false }")
            && breach.abilities[0].contains(
                "Effect::ModifyEvent { modifier: Modifier::AddKeyword(KeywordSet::HASTE), duration: Duration::Indefinitely }"
            )
            && breach.abilities[0].contains(
                "Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::EndStep, whose: None, this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EventObject, effects: &[Effect::SacrificeEvent] }"
            ),
        "{:?}",
        breach.abilities
    );

    // Put Equipment from hand onto battlefield + optional MoveAttachment.
    let equip_drop = read_w26d(
        "Name:Forge Drop\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Equipment.YouOwn | RememberChanged$ True | Optional$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ Remembered | Choices$ Creature.YouCtrl | Optional$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        equip_drop.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && equip_drop.abilities[0].contains(
                "Effect::MoveAttachment { attachment: TargetSpec::EventObject, \
                 recipient_filter: &Filter::YOUR_CREATURE, must_change_recipient: false, \
                 optional: true, then_if_moved: &[] }"
            ),
        "{:?}",
        equip_drop.abilities
    );

    // Temur Sabertooth shape: Battlefield -> Hand Hidden without Optional$ (Oracle "You may") + RememberChanged.
    let sabertooth = read_w26d(
        "Name:Bounce Cat\nManaCost:2 G G\nTypes:Creature\nPT:4/3\n\
         A:AB$ ChangeZone | Cost$ 1 G | Origin$ Battlefield | Destination$ Hand | Hidden$ True | ChangeType$ Creature.YouCtrl+Other | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ1 | Defined$ Self | KW$ Indestructible | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:{1}{G}: You may return another creature you control to its owner's hand. If you do, Bounce Cat gains indestructible until end of turn.",
    );
    assert!(
        sabertooth.abilities[0].contains(
            "Effect::ChooseCard { chooser: PlayerRel::You, zone: ChooseZone::Battlefield"
        ) && sabertooth.abilities[0].contains(
            "Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ReturnedToHand(&Filter::Any)), 1)"
        ),
        "{:?}",
        sabertooth.abilities
    );

    // Entrails Feaster shape: Hidden Graveyard -> Exile Optional + RememberChanged with GE1 and LE0 branches.
    let feaster = read_w26d(
        "Name:Grave Eater\nManaCost:B\nTypes:Creature Zombie\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigExile | TriggerZones$ Battlefield\n\
         SVar:TrigExile:DB$ ChangeZone | ChangeType$ Creature | ChangeNum$ 1 | Origin$ Graveyard | Destination$ Exile | Optional$ True | IsCurse$ True | RememberChanged$ True | Hidden$ True | Chooser$ You | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Self | CounterNum$ 1 | CounterType$ P1P1 | ConditionCheckSVar$ X | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | ConditionCheckSVar$ X | ConditionSVarCompare$ LE0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        feaster.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)")
            && feaster.abilities[0].contains("Effect::TapSelf"),
        "{:?}",
        feaster.abilities
    );

    // Player-targeted ChangeZoneAll Battlefield -> Hand / Exile (`TargetedPlayerCtrl`).
    let bounce_opp = read_w26d(
        "Name:Tidal Sweep\nManaCost:3 U U\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Opponent | Origin$ Battlefield | Destination$ Hand | ChangeType$ Creature.TargetedPlayerCtrl",
    );
    assert!(
        bounce_opp.abilities[0].contains("Effect::ReturnAllToHand { filter: &BOUNCE_ALL1")
            && bounce_opp
                .statics
                .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{:?} / {}",
        bounce_opp.abilities,
        bounce_opp.statics
    );

    // Wedding Ring shape: CopyPermanent Defined$ Self + ValidTgts$ Opponent + Controller$ TargetedPlayer.
    let wedding = read_w26d(
        "Name:Vow Ring\nManaCost:2 W W\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopyPermanent | ValidTgts$ Opponent | Defined$ Self | Controller$ TargetedPlayer",
    );
    assert!(
        wedding.abilities[0].contains(
            "Effect::CreateTokenCopy { target: TargetSpec::ThisObject, controller: PlayerRel::Chosen, \
             amount: Amount::Fixed(1), mods: &[], tapped: false, attacking: false, \
             haste_until_eot: false, cleanup: CopyCleanup::None }"
        ),
        "{:?}",
        wedding.abilities
    );

    // Optional CopyPermanent + Defined$ Enchanted.
    let opt_copy = read_w26d(
        "Name:Mirror Aura\nManaCost:2 U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopyPermanent | Defined$ Enchanted | Optional$ True",
    );
    assert!(
        opt_copy.abilities.iter().any(|a| a.contains(
            "Effect::MayDo { effects: &[Effect::CreateTokenCopy { target: TargetSpec::Object(&Filter::AttachedToBySource)"
        )),
        "{:?}",
        opt_copy.abilities
    );

    // Kaldheim Equipment token-attach shape (`Object$ TriggeredCardLKICopy`).
    let runed_gear = read_w26d(
        "Name:Runed Hammer\nManaCost:2 R\nTypes:Artifact Equipment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigToken | TriggerZones$ Battlefield\n\
         SVar:TrigToken:AB$ Token | Cost$ 2 | TokenScript$ r_1_1_goblin | RememberTokens$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ TriggeredCardLKICopy | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        runed_gear.abilities[0].contains("Effect::CreateTokenThenAttachSelf"),
        "{:?}",
        runed_gear.abilities
    );
}

#[test]
fn wave26d_unbounded_chosen_number_binds_discard_mana_value() {
    let choice = read_w26d(
        "Name:Numbered Discard\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChooseNumber | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | ValidTgts$ Player | Mode$ RevealDiscardAll | DiscardValid$ Card.nonLand+cmcEQChosen\n\
         SVar:Chosen:Count$ChosenNumber",
    );
    let text = format!("{}\n{}", choice.abilities.join("\n"), choice.statics);
    assert!(text.contains("max: Amount::Fixed(u32::MAX)"), "{text}");
    assert!(
        text.contains("Effect::RevealHand { who: PlayerRel::Chosen }"),
        "{text}"
    );
    assert!(text.contains("Effect::DiscardMatching"), "{text}");
    assert!(
        text.contains("Filter::CmcEqualsAmount(&Amount::ResolutionChosenNumber)"),
        "{text}"
    );
}

#[test]
fn wave26d_honest_refusals_and_pure_paper_guardrails() {
    for bad in [
        // Alchemy / digital mechanics must stay refused.
        "Name:BadPerpetual\nManaCost:1 U\nTypes:Sorcery\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | Duration$ Perpetual",
        "Name:BadConjure\nManaCost:1 U\nTypes:Sorcery\nA:SP$ MakeCard | Name$ Lightning Bolt",
        // Un-set / Sticker / Planechase mechanics must stay refused.
        "Name:BadSticker\nManaCost:1 W\nTypes:Sorcery\nA:SP$ PutSticker | ValidTgts$ Creature",
        // Mismatched SacX filter on SacrificeAnyNumber must refuse.
        "Name:BadSacX\nManaCost:1 W\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Land | RememberSacrificed$ True | Optional$ True | SubAbility$ DBDraw\nSVar:DBDraw:DB$ Draw | NumCards$ Y | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nSVar:SacX:Count$Valid Creature.YouCtrl\nSVar:Y:Remembered$Amount",
        // Mandatory dynamic-count graveyard return (Lich-Knights' Conquest) must stay refused until dynamic min exists.
        "Name:BadMandatoryDynamicGy\nManaCost:4 B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Artifact | RememberSacrificed$ True | Optional$ True | SubAbility$ DBRet\nSVar:DBRet:DB$ ChangeZone | ChangeType$ Creature.YouOwn | Origin$ Graveyard | Destination$ Battlefield | Mandatory$ True | ChangeNum$ Y | Hidden$ True | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nSVar:SacX:Count$Valid Artifact.YouCtrl\nSVar:Y:Remembered$Amount",
        // ChooseNumber with Secretly$ True or conflicting UnlessCost$ PayEnergy must refuse.
        "Name:BadSecretNum\nManaCost:2 U\nTypes:Sorcery\nA:SP$ ChooseNumber | Defined$ You | Min$ 0 | Max$ 5 | Secretly$ True | SubAbility$ DBDraw\nSVar:DBDraw:DB$ Draw | NumCards$ X\nSVar:X:Count$ChosenNumber",
        "Name:BadDoubleEnergyPay\nManaCost:R\nTypes:Instant\nA:SP$ ChooseNumber | Max$ Max | ListTitle$ amount of energy to pay | SubAbility$ DBDmg\nSVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ X | UnlessCost$ PayEnergy<X> | UnlessPayer$ You | UnlessSwitched$ True\nSVar:Max:Count$YourCountersEnergy\nSVar:X:Count$ChosenNumber",
        // GainControlAllEx with non-Haste AddKWs or non-EOT Haste must refuse.
        "Name:BadGainControlKw\nManaCost:5 R\nTypes:Sorcery\nA:SP$ GainControl | AllValid$ Creature | AddKWs$ Trample | LoseControl$ EOT",
        "Name:BadGainControlPermHaste\nManaCost:5 R\nTypes:Sorcery\nA:SP$ GainControl | AllValid$ Creature | AddKWs$ Haste",
        // MoveAttachment with PlayerChoices$ must refuse.
        "Name:BadAttachPlayer\nManaCost:2 W\nTypes:Sorcery\nA:SP$ RepeatEach | RepeatCards$ Aura.YouCtrl | RepeatSubAbility$ DBAttach\nSVar:DBAttach:DB$ Attach | Object$ Remembered | PlayerChoices$ Opponent",
        // CopyPermanent with unsupported AtEOT$ YourSacrifice must refuse.
        "Name:BadYourSacCopy\nManaCost:3 R\nTypes:Sorcery\nA:SP$ CopyPermanent | ValidTgts$ Creature.YouCtrl | AtEOT$ YourSacrifice",
        // Player-targeted ChangeZoneAll that ignores the player target must refuse.
        "Name:BadUnusedPlayerSweep\nManaCost:3 U\nTypes:Sorcery\nA:SP$ ChangeZoneAll | ValidTgts$ Opponent | Origin$ Battlefield | Destination$ Hand | ChangeType$ Creature",
        // Event$ Counter with non-CantHappen layer must refuse.
        "Name:BadCounterLayer\nManaCost:G\nTypes:Instant\nA:SP$ Effect | ReplacementEffects$ Anti\nSVar:Anti:Event$ Counter | ValidSA$ Spell.YouCtrl | Layer$ Other",
    ] {
        assert!(refused_w26d(bad), "must refuse: {bad}");
    }
}
