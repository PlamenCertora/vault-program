mod tests {
    use crate::utils::*;

    // Test LP Minting Functions
    #[test]
    fn test_calc_init_lp_to_mint() {
        // Same decimals
        assert_eq!(calc_init_lp_to_mint(1000, 9, 9).unwrap(), 1000);

        // Different decimals - 6 to 9 (multiply by 1000)
        assert_eq!(calc_init_lp_to_mint(1000, 6, 9).unwrap(), 1_000_000);

        // Different decimals - 9 to 6 (divide by 1000)
        assert_eq!(calc_init_lp_to_mint(1_000_000, 9, 6).unwrap(), 1000);

        // Edge case - zero amount
        assert_eq!(calc_init_lp_to_mint(0, 6, 9).unwrap(), 0);

        // Large amounts
        let large_amount = 1_000_000_000; // 1B
        assert_eq!(
            calc_init_lp_to_mint(large_amount, 6, 9).unwrap(),
            large_amount * 1000
        );
    }

    #[test]
    fn test_calc_deposit_lp_to_mint() {
        // No fee scenario
        let lp_minted = calc_deposit_lp_to_mint(1000, 10000, 5000, 0).unwrap();
        // Expected: (1000 * 10000 * 10000) / (6000 * 10000 - 1000 * 10000) = 2000
        assert_eq!(lp_minted, 2000);

        // With 1% issuance fee (100 bps)
        let lp_minted_with_fee = calc_deposit_lp_to_mint(1000, 10000, 5000, 100).unwrap();
        // Fee reduces LP minted
        assert!(lp_minted_with_fee < lp_minted);

        // Edge case - very small amounts
        let small_lp = calc_deposit_lp_to_mint(1, 1000000, 500000, 0).unwrap();
        assert_eq!(small_lp, 2); // Should still work with small amounts
    }

    #[test]
    fn test_calc_fee_lp_to_mint() {
        // Basic fee calculation
        let fee_lp = calc_fee_lp_to_mint(100, 1000, 2000).unwrap();
        // Expected: ceil(100 * 1000 / (2000 - 100)) = ceil(52.63) = 53
        assert_eq!(fee_lp, 53);

        // Zero fee
        assert_eq!(calc_fee_lp_to_mint(0, 1000, 2000).unwrap(), 0);
    }

    // Test Withdrawal Functions
    #[test]
    fn test_calc_withdraw_lp_to_burn() {
        // No redemption fee
        let redeem_amount = to_decimal_bits(1000);
        let lp_to_burn = calc_withdraw_lp_to_burn(redeem_amount, 2000, 1000, 0).unwrap();
        // Expected: ceil(1000 * (2000/1000) * (10000/10000)) = 2000
        assert_eq!(lp_to_burn, 2000);

        // With 1% redemption fee (100 bps)
        let lp_with_fee = calc_withdraw_lp_to_burn(redeem_amount, 2000, 1000, 100).unwrap();
        // Fee increases LP to burn
        assert!(lp_with_fee > lp_to_burn);
    }

    #[test]
    fn test_calc_withdraw_asset_to_redeem() {
        // No redemption fee
        let asset_bits = calc_withdraw_asset_to_redeem(1000, 2000, 1000, 0).unwrap();
        let asset_amount = Decimal::from_bits(asset_bits).to_floor::<u64>();
        // Expected: 1000 * (1000/2000) * (10000/10000) = 500
        assert_eq!(asset_amount, 500);

        // With 2% redemption fee (200 bps)
        let asset_with_fee = calc_withdraw_asset_to_redeem(1000, 2000, 1000, 200).unwrap();
        let asset_amount_with_fee: u64 = Decimal::from_bits(asset_with_fee).to_floor();
        // Fee reduces asset to redeem: 500 * 0.98 = 490
        assert_eq!(asset_amount_with_fee, 490);
    }

    #[test]
    fn test_get_capped_withdraw_asset_to_redeem() {
        let initial_amount = to_decimal_bits(600);

        // Case 1: Latest amount is less than initial - should return latest
        let capped =
            get_capped_withdraw_asset_to_redeem(initial_amount, 1000, 2000, 1000, 0).unwrap();
        assert_eq!(capped, 500); // Latest calculation result

        // Case 2: Latest amount is more than initial - should return initial
        let capped_higher =
            get_capped_withdraw_asset_to_redeem(to_decimal_bits(200), 1000, 2000, 1000, 0).unwrap();
        assert_eq!(capped_higher, 200); // Initial amount (capped)
    }

    // Test Complex Withdrawal Logic
    #[test]
    fn test_get_amount_lp_to_burn_and_asset_to_withdraw() {
        // Withdraw all scenario
        let (lp_burn, asset_withdraw) =
            get_amount_lp_to_burn_and_asset_to_withdraw(0, 1000, 5000, 2500, 100, false, true)
                .unwrap();
        assert_eq!(lp_burn, 1000); // All user LP
        let asset_amount = Decimal::from_bits(asset_withdraw).to_floor::<u64>();
        assert!(asset_amount > 0); // Should get some assets back

        // Amount in LP tokens
        let (lp_burn, _) =
            get_amount_lp_to_burn_and_asset_to_withdraw(500, 1000, 5000, 2500, 100, true, false)
                .unwrap();
        assert_eq!(lp_burn, 500); // Exact LP amount specified

        // Amount in asset tokens
        let (lp_burn, asset_withdraw) =
            get_amount_lp_to_burn_and_asset_to_withdraw(250, 1000, 5000, 2500, 100, false, false)
                .unwrap();
        assert!(lp_burn > 0); // Should calculate LP to burn
        assert_eq!(Decimal::from_bits(asset_withdraw).to_floor::<u64>(), 250);
    }

    #[test]
    fn test_calc_cancel_withdraw_amount_lp_to_refund_and_burn() {
        // Basic scenario
        let initial_asset_worth = to_decimal_bits(500);
        let (refund, burn) = calc_cancel_withdraw_amount_lp_to_refund_and_burn(
            1000,
            initial_asset_worth,
            5000,
            2500,
            100,
        )
        .unwrap();

        assert_eq!(refund + burn, 1000); // Total should equal initial

        // Edge case: vault improved, user gets less back
        assert!(refund <= 1000);

        // Edge case: LP supply equals initial refund
        let (refund_edge, burn_edge) = calc_cancel_withdraw_amount_lp_to_refund_and_burn(
            1000,
            initial_asset_worth,
            1000,
            2500,
            100,
        )
        .unwrap();
        assert_eq!(refund_edge, 1000);
        assert_eq!(burn_edge, 0);
    }

    #[test]
    fn test_realistic_scenarios() {
        // Scenario: User deposits 1000 USDC into vault with 10M USDC and 20M LP tokens
        // with 0.25% issuance fee
        let deposit_lp = calc_deposit_lp_to_mint(
            1_000_000_000,          // 1000 USDC (6 decimals)
            20_000_000_000_000_000, // 20M LP tokens
            10_000_000_000_000,     // 10M USDC already in vault
            25,                     // 0.25% fee
        )
        .unwrap();

        println!("deposit_lp: {:?}", deposit_lp);

        // (1000000000*(10000-25)*20000000000000000)/(10000*(1000000000+10000000000000) - 1000000000*(10000-25)) (round down)
        assert_eq!(deposit_lp, 1994999501250);

        // Scenario: Withdraw 1000 USDC worth with 0.5% redemption fee
        let withdraw_lp = calc_withdraw_lp_to_burn(
            to_decimal_bits(1_000_000_000), // 1000 USDC
            20_000_000_000_000_000,         // 20M LP
            10_000_000_000_000,             // 10M USDC
            50,                             // 0.5% redemption fee
        )
        .unwrap();

        // lp_to_burn = 1000000000 * (20000000000000000 / 10000000000000) = 2000000000000
        // lp_to_burn_post_fee = 2000000000000 * (10000 / (10000 - 50)) = 2010050251257 (round up)
        assert_eq!(withdraw_lp, 2010050251257);
    }
}
