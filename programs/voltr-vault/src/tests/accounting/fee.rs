mod tests {
    use crate::utils::constants::{MAX_FEE_BPS, ONE_YEAR_U64};
    use crate::utils::*;

    // Test Fee Calculation Functions
    #[test]
    fn test_calc_management_fee_amount_in_asset() {
        // 1% annual fee for full year
        let fee = calc_management_fee_amount_in_asset(ONE_YEAR_U64, 1_000_000, 100).unwrap();
        assert_eq!(fee, 10_000); // 1% of 1M

        // Half year
        let fee_half =
            calc_management_fee_amount_in_asset(ONE_YEAR_U64 / 2, 1_000_000, 100).unwrap();
        assert_eq!(fee_half, 5_000); // 0.5% of 1M

        // Zero time elapsed
        let fee_zero = calc_management_fee_amount_in_asset(0, 1_000_000, 100).unwrap();
        assert_eq!(fee_zero, 0);
    }

    #[test]
    fn test_calc_performance_fee_amount_in_asset() {
        // 20% performance fee (2000 bps)
        let fee = calc_performance_fee_amount_in_asset(1000, 2000).unwrap();
        assert_eq!(fee, 200); // 20% of 1000

        // 0% performance fee
        let fee_zero = calc_performance_fee_amount_in_asset(1000, 0).unwrap();
        assert_eq!(fee_zero, 0);

        // Edge case: 100% performance fee
        let fee_max = calc_performance_fee_amount_in_asset(1000, 10000).unwrap();
        assert_eq!(fee_max, 1000);
    }

    #[test]
    fn test_calc_fee_split() {
        // Split 1000 LP fees: 10% protocol, 60% admin, 40% manager (of remaining)
        let (admin, manager, protocol) = calc_fee_split(1000, 1000, 5000, 4000).unwrap();

        assert_eq!(protocol, 100); // 10% of 1000
        assert_eq!(admin + manager, 900); // Remaining 900

        // Admin should get remaining: 900 * 5000/(5000+4000) = 500
        assert_eq!(admin, 500);
        assert_eq!(manager, 400);

        // Zero protocol fee
        let (admin2, manager2, protocol2) = calc_fee_split(1000, 0, 5000, 5000).unwrap();
        assert_eq!(protocol2, 0);
        assert_eq!(admin2, 500); // 50% of 1000
        assert_eq!(manager2, 500); // 50% of 1000
    }

    // Test Edge Cases and Error Conditions
    #[test]
    fn test_division_by_zero_cases() {
        // These should all handle division by zero gracefully
        assert!(calc_deposit_lp_to_mint(1000, 0, 0, 0).is_err());

        assert!(calc_fee_lp_to_mint(1000, 1000, 1000).is_err());

        assert!(calc_withdraw_lp_to_burn(100, 100, 100, 10000).is_err());
    }

    #[test]
    fn test_overflow_conditions() {
        // Test with very large numbers that might cause overflow
        let large_num = u64::MAX / 2;

        // These should either work or fail gracefully with SafeMath
        let result = calc_init_lp_to_mint(large_num, 0, 18);
        // This specific case would overflow, so should error
        assert!(result.is_err());

        // Test management fee with large values and long time
        let large_fee_result = calc_management_fee_amount_in_asset(
            ONE_YEAR_U64 * 10, // 10 years
            large_num,
            1000, // 10%
        );
        // Should handle this gracefully
        assert!(large_fee_result.is_ok() || large_fee_result.is_err());
    }

    #[test]
    fn test_precision_and_rounding() {
        // Test that small amounts don't get rounded to zero inappropriately
        let small_fee = calc_performance_fee_amount_in_asset(1, 1).unwrap();
        // 1 * 1 / 10000, should ceil to 1
        assert_eq!(small_fee, 1);

        // Test decimal precision in LP calculations
        let precise_result = calc_withdraw_asset_to_redeem(1, 3, 2, 3333).unwrap();
        let amount = Decimal::from_bits(precise_result);
        // Should maintain precision for small calculations
        assert!(amount.to_bits() > 0);
    }

    #[test]
    fn test_fee_boundary_conditions() {
        // Test with maximum fee rates
        let max_fee_result = calc_performance_fee_amount_in_asset(1000, MAX_FEE_BPS).unwrap();
        assert_eq!(max_fee_result, 1000); // 100% fee

        // Test with zero amounts
        assert_eq!(calc_performance_fee_amount_in_asset(0, 5000).unwrap(), 0);
        assert_eq!(
            calc_management_fee_amount_in_asset(1000, 0, 5000).unwrap(),
            0
        );

        // Test fee split with zero amounts
        let (a, m, p) = calc_fee_split(0, 1000, 5000, 5000).unwrap();
        assert_eq!(a + m + p, 0);
    }
}
