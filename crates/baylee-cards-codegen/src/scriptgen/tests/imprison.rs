//! Original synthetic grammar fixtures, not external script copies.
use super::*;
fn clause(mode: &str) -> String {
    let (valid, extra, paid, tail) = match mode {
        "AbilityCast" => ("Creature.EnchantedBy", " | ValidSA$ SpellAbility.!ManaAbility+hasTapCost", "DB$ Counter | Defined$ TriggeredSpellAbility".to_string(), String::new()),
        "Attacks" => ("Card.EnchantedBy", "", "DB$ Tap | Defined$ TriggeredAttackerLKICopy | SubAbility$ EndAttacks".into(), "SVar:EndAttacks:DB$ RemoveFromCombat | Defined$ TriggeredAttackerLKICopy\n".into()),
        "Blocks" => ("Card.EnchantedBy", " | Secondary$ True", "DB$ Tap | Defined$ TriggeredBlockerLKICopy | SubAbility$ EndBlocks".into(), "SVar:EndBlocks:DB$ RemoveFromCombat | Defined$ TriggeredBlockerLKICopy | UnblockCreaturesBlockedOnlyBy$ TriggeredBlocker\n".into()),
        _ => unreachable!(),
    };
    format!(
        "T:Mode$ {mode} | ValidCard$ {valid}{extra} | TriggerZones$ Battlefield | Execute$ Invoice{mode}\nSVar:Invoice{mode}:DB$ Destroy | Defined$ Self | UnlessCost$ 1 | UnlessPayer$ You | UnlessResolveSubs$ WhenPaid | SubAbility$ Paid{mode}\nSVar:Paid{mode}:{paid}\n{tail}"
    )
}
fn script() -> String {
    let mut text = "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n".to_string();
    for mode in ["AbilityCast", "Attacks", "Blocks"] {
        text.push_str(&clause(mode));
    }
    text
}
#[test]
fn enchanted_creature_payments_read_all_three_clauses_with_the_correct_payer() {
    let body = read(&script());
    assert_eq!(
        body.abilities.len(),
        4,
        "Aura spell plus all three triggers"
    );
    let all = body.abilities.join(" ");
    assert_eq!(all.matches("PlayerMayPayThenElse").count(), 3);
    assert_eq!(all.matches("player: PlayerRel::You").count(), 3);
    assert_eq!(
        all.matches("Effect::destroy(TargetSpec::ThisObject)")
            .count(),
        3
    );
    assert!(all.contains("Trigger::AbilityActivated"));
    assert!(all.contains("tap_cost: Some(true)"));
    assert!(all.contains("mana_ability: Some(false)"));
    assert!(all.contains("Effect::CounterEventAbility"));
    assert!(all.contains("Trigger::Attacks(&Filter::AttachedToBySource)"));
    assert!(all.contains("Trigger::Blocks(&Filter::AttachedToBySource)"));
    assert_eq!(
        all.matches("RemoveEventObjectFromCombat { unblock: true }")
            .count(),
        2
    );
}
#[test]
fn enchanted_creature_reader_refuses_unclaimed_trigger_payment_and_child_rules() {
    assert!(
        refused("Name:X\nTypes:Instant\nA:SP$ RemoveFromCombat | ValidTgts$ Creature"),
        "contextual support must not admit arbitrary chains"
    );
    let text = script();
    for bad in [
        text.replace("!ManaAbility+hasTapCost", "ManaAbility+hasTapCost"),
        text.replace("!ManaAbility+hasTapCost", "!ManaAbility"),
        text.replace("TriggerZones$ Battlefield", "TriggerZones$ Graveyard"),
        text.replace("UnlessCost$ 1", "UnlessCost$ 2"),
        text.replace("UnlessPayer$ You", "UnlessPayer$ TriggeredActivator"),
        text.replace("UnlessResolveSubs$ WhenPaid", "UnlessResolveSubs$ Always"),
        text.replace("Defined$ TriggeredSpellAbility", "Defined$ Targeted"),
        text.replace(
            "Defined$ TriggeredAttackerLKICopy",
            "Defined$ TriggeredAttacker",
        ),
        text.replace(
            "UnblockCreaturesBlockedOnlyBy$ TriggeredBlocker",
            "UnblockCreaturesBlockedOnlyBy$ All",
        ),
        text.replace(" | UnblockCreaturesBlockedOnlyBy$ TriggeredBlocker", ""),
        text.replace("Secondary$ True", "Secondary$ False"),
        text.replace(
            "DB$ Counter | Defined$ TriggeredSpellAbility",
            "DB$ Counter | Defined$ TriggeredSpellAbility | NoRegen$ True",
        ),
        text.replace(
            "Execute$ InvoiceBlocks",
            "Execute$ InvoiceBlocks | OptionalDecider$ You",
        ),
    ] {
        assert!(refused(&bad), "{bad}");
    }
}
