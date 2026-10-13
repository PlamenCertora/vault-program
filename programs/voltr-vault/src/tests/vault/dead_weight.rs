#[cfg(test)]
mod tests {
    use crate::state::vault::*;
    use crate::utils::constants::DEAD_WEIGHT;

    #[test]
    fn test_dead_weight_attribution_lifecycle() {
        let mut vault = Vault::default();

        // 1. Verify initial state
        assert_eq!(vault.dead_weight, 0);

        // 2. Test First Deposit: Insufficient Amount
        // Attempting to mint less than DEAD_WEIGHT should fail
        let insufficient_amount = DEAD_WEIGHT - 1;
        assert!(vault
            .handle_dead_weight_attribution_idempotent(insufficient_amount)
            .is_err());
        assert_eq!(vault.dead_weight, 0);

        // 3. Test First Deposit: Sufficient Amount
        // Should deduct DEAD_WEIGHT from the user's LP and permanently set vault.dead_weight
        let initial_mint_amount = DEAD_WEIGHT + 5000;
        let lp_to_user = vault
            .handle_dead_weight_attribution_idempotent(initial_mint_amount)
            .unwrap();

        assert_eq!(vault.dead_weight, DEAD_WEIGHT);
        assert_eq!(lp_to_user, 5000); // 6000 - 1000 = 5000

        // 4. Test Subsequent Deposit
        // Should NOT deduct DEAD_WEIGHT again. User gets full amount.
        let second_mint_amount = 2000;
        let lp_to_user_2 = vault
            .handle_dead_weight_attribution_idempotent(second_mint_amount)
            .unwrap();

        assert_eq!(vault.dead_weight, DEAD_WEIGHT); // Remains constant
        assert_eq!(lp_to_user_2, second_mint_amount); // User gets full 2000

        // 5. Verify Total Supply Calculation
        // The total supply calculation must account for the burned dead weight
        let current_user_supply = lp_to_user + lp_to_user_2; // 7000
        let total_supply_incl_fees = vault
            .get_total_lp_supply_incl_fees(current_user_supply)
            .unwrap();

        // Total = User Supply (7000) + Dead Weight (1000) = 8000
        assert_eq!(total_supply_incl_fees, current_user_supply + DEAD_WEIGHT);
    }
}
