//! Mana produced while playing an instructed card must pay that play or its land abilities.

use crate::state::GameState;
use baylee_core::ids::{DamageSourceRef, PlayerId};
use baylee_core::mana::{ManaColor, ManaFlags, ManaPool};

/// One nested, exact-card mana obligation. Entries preserve snow and restrictions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ConstrainedPayment {
    pub(crate) player: PlayerId,
    pub(crate) card: DamageSourceRef,
    pub(crate) required: ManaPool,
}

impl GameState {
    pub(crate) fn constrained_payment(&self, player: PlayerId) -> Option<&ConstrainedPayment> {
        self.constrained_payments
            .iter()
            .rev()
            .find(|payment| payment.player == player)
    }

    /// Called around one mana-producing instruction, never around a payment.
    pub(crate) fn note_constrained_production(&mut self, player: PlayerId, before: &ManaPool) {
        let after = &self.players[usize::from(player.get())].mana_pool;
        let Some(mut produced) = after.payment_receipt(before) else {
            return;
        };
        let Some(payment) = self
            .constrained_payments
            .iter_mut()
            .rev()
            .find(|payment| payment.player == player)
        else {
            return;
        };
        // The obligation is a subset of the real pool; the real production
        // already checked its numeric capacity.
        let transferred = produced.transfer_to(&mut payment.required);
        if !transferred || amounts(&payment.required).is_none() {
            self.numeric_failure = Some("generated-mana obligation exceeds its numeric domain");
        }
    }

    /// Debit only units actually consumed, preferring obligated fungible units.
    pub(crate) fn note_constrained_payment(&mut self, player: PlayerId, before: &ManaPool) {
        let after = &self.players[usize::from(player.get())].mana_pool;
        let Some(receipt) = before.payment_receipt(after) else {
            return;
        };
        let Some(payment) = self
            .constrained_payments
            .iter_mut()
            .rev()
            .find(|payment| payment.player == player)
        else {
            return;
        };
        for entry in receipt.restricted() {
            let _ = payment
                .required
                .take_restricted_units(entry.restriction.0, entry.amount);
        }
        for color in ManaColor::ALL {
            let total = receipt
                .available(color)
                .min(payment.required.available(color));
            let snow = receipt
                .snow_available(color)
                .min(payment.required.snow_available(color))
                .min(total);
            // Core exposes an O(1) snow debit beside ordinary spending.
            let _ = payment.required.spend_snow_units(color, snow);
            let _ = payment.required.spend(color, total - snow);
        }
    }
}

/// Total obligated units of each type, including restricted entries once.
pub(crate) fn amounts(pool: &ManaPool) -> Option<[u32; 6]> {
    let mut amounts = [0; 6];
    for color in ManaColor::ALL {
        amounts[color.index()] = pool
            .restricted()
            .iter()
            .filter(|unit| unit.color == color)
            .try_fold(pool.available(color), |sum, unit| {
                sum.checked_add(u32::from(unit.amount))
            })?;
    }
    Some(amounts)
}

/// The snow subset of restricted units is charged separately by the payment reader.
pub(crate) fn restricted_snow(pool: &ManaPool, color: ManaColor) -> u64 {
    pool.restricted()
        .iter()
        .filter(|unit| unit.color == color && unit.flags.contains(ManaFlags::SNOW))
        .map(|unit| u64::from(unit.amount))
        .sum()
}
