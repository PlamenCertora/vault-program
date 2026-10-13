mod tests {
    use crate::state::vault::*;
    use crate::utils::Decimal;

    #[test]
    fn test_high_water_mark_calibration() {
        let mut hwm = HighWaterMark::default();

        // Test with zero LP supply - should not panic
        assert!(hwm.calibrate_high_water_mark(1000, 0, 1704067200).is_ok());
        assert_eq!(
            hwm.highest_asset_per_lp_decimal_bits,
            Decimal::ZERO.to_bits()
        );

        // Test with normal values
        hwm.calibrate_high_water_mark(2000, 1000, 1704067200)
            .unwrap();
        assert_ne!(
            hwm.highest_asset_per_lp_decimal_bits,
            Decimal::ZERO.to_bits()
        );
    }

    #[test]
    fn test_high_water_mark_updates() {
        let mut hwm = HighWaterMark::default();
        hwm.calibrate_high_water_mark(2000, 1000, 1704067200)
            .unwrap();
        let initial_bits = hwm.highest_asset_per_lp_decimal_bits;

        // Test idempotent update with same values
        hwm.update_high_water_mark_idempotent(2000, 1000, 1704067200)
            .unwrap();
        assert_eq!(hwm.highest_asset_per_lp_decimal_bits, initial_bits);

        // Test update with lower ratio - should not update
        hwm.update_high_water_mark_idempotent(1500, 1000, 1704067300)
            .unwrap();
        assert_eq!(hwm.highest_asset_per_lp_decimal_bits, initial_bits);
    }

    #[test]
    fn test_eligible_profit_calculation() {
        let mut hwm = HighWaterMark::default();

        // Test with zero HWM
        assert_eq!(hwm.calculate_eligible_profit(1000, 1000).unwrap(), 1000);

        // Test with established HWM
        hwm.calibrate_high_water_mark(2000, 1000, 1704067200)
            .unwrap();
        assert_eq!(hwm.calculate_eligible_profit(1500, 1000).unwrap(), 0); // Lower than HWM
        assert!(hwm.calculate_eligible_profit(2500, 1000).unwrap() > 0); // Higher than HWM
    }
}
