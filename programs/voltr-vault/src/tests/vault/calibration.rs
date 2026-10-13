#[cfg(test)]
mod tests {
    use crate::state::vault::*;
    use crate::utils::{Decimal, SafeMath};

    #[test]
    fn test_calibrate_hwm_initial_increase() {
        let mut vault = Vault::default();
        let current_ts = 100_000;

        // 1000 assets, 1000 LP -> 1.0 price
        vault.asset.total_value = 1000;
        let lp_supply = 1000;

        // HWM is 0 initially.
        // Current (1.0) > HWM (0) -> No "reverted profit".
        // No fee, no burn.

        let (assets, supply_before, hwm_before, hwm_after, burn) = vault
            .calibrate_high_water_mark(lp_supply, current_ts)
            .unwrap();

        assert_eq!(assets, 1000);
        assert_eq!(supply_before, 1000);
        assert_eq!(burn, 0);
        assert_eq!(hwm_before, 0); // Decimal::ZERO
        assert_eq!(hwm_after, Decimal::ONE.to_bits());
        assert_eq!(vault.high_water_mark.last_updated_ts, current_ts);
    }

    #[test]
    fn test_calibrate_hwm_drawdown_no_fee() {
        let mut vault = Vault::default();
        let current_ts = 100_000;

        // HWM at 2.0
        vault.high_water_mark.highest_asset_per_lp_decimal_bits = Decimal::from_num(2).to_bits();

        // Current: 1000 assets, 1000 LP -> 1.0 price
        vault.asset.total_value = 1000;
        let lp_supply = 1000;

        // No fees configured.

        let (_assets, _supply_before, hwm_before, hwm_after, burn) = vault
            .calibrate_high_water_mark(lp_supply, current_ts)
            .unwrap();

        assert_eq!(burn, 0);
        assert_eq!(hwm_before, Decimal::from_num(2).to_bits());

        // Since burn is 0, we calibrate HWM to current price: 1000 assets / 1000 LP = 1.0.
        assert_eq!(hwm_after, Decimal::ONE.to_bits());
        assert_eq!(vault.high_water_mark.last_updated_ts, current_ts);
    }

    #[test]
    fn test_calibrate_hwm_drawdown_with_fee_burn() {
        let mut vault = Vault::default();
        let current_ts = 100_000;

        // HWM at 2.0
        vault.high_water_mark.highest_asset_per_lp_decimal_bits = Decimal::from_num(2).to_bits();

        // Current: 1000 assets, 1000 LP -> 1.0 price
        vault.asset.total_value = 1000;
        let lp_supply = 1000;

        // Fee: 10% performance fee (1000 bps)
        vault.fee_configuration.manager_performance_fee = 1000;

        // Logic Check:
        // Reverted Profit (Loss relative to HWM) = (HWM - Current) * LP
        // = (2.0 - 1.0) * 1000 = 1000.
        // Fee Amount = 1000 * 10% = 100.
        // LP Burn = Fee(100) * LP(1000) / Assets(1000) = 100.

        let (_assets, _supply_before, _hwm_before, hwm_after, burn) = vault
            .calibrate_high_water_mark(lp_supply, current_ts)
            .unwrap();

        assert_eq!(burn, 100);

        // Post-burn logic for HWM update:
        // New Supply = 1000 - 100 = 900.
        // Assets = 1000.
        // New HWM = 1000 / 900 = 1.111...

        let expected_hwm = Decimal::from_num(1000)
            .safe_div(Decimal::from_num(900))
            .unwrap();
        assert_eq!(hwm_after, expected_hwm.to_bits());
    }

    #[test]
    fn test_calibrate_with_accumulated_lp() {
        let mut vault = Vault::default();
        let current_ts = 100_000;

        // Supply excl fees: 900
        let lp_supply_excl = 900;
        // Fees: 100
        vault.fee_state.accumulated_lp_admin_fees = 100;
        // Total supply = 1000.

        // Assets = 1000. Price = 1.0.
        vault.asset.total_value = 1000;

        // HWM = 2.0.
        vault.high_water_mark.highest_asset_per_lp_decimal_bits = Decimal::from_num(2).to_bits();

        // Fee 10%.
        vault.fee_configuration.manager_performance_fee = 1000;

        // Same calculation as before:
        // Loss = (2.0 - 1.0) * 1000 = 1000.
        // Fee = 100.
        // Burn = 100 * 1000 / 1000 = 100.

        let (assets, supply_incl, _, _, burn) = vault
            .calibrate_high_water_mark(lp_supply_excl, current_ts)
            .unwrap();

        assert_eq!(assets, 1000);
        assert_eq!(supply_incl, 1000);
        assert_eq!(burn, 100);
    }

    #[test]
    fn test_update_hwm_idempotent() {
        let mut vault = Vault::default();
        let current_ts = 100_000;

        // 1. Set initial HWM at 1.0
        vault.high_water_mark.highest_asset_per_lp_decimal_bits = Decimal::ONE.to_bits();

        // 2. Case: Current < HWM (0.5)
        vault.asset.total_value = 500;
        let lp_supply = 1000;

        vault
            .update_high_water_mark_idempotent(lp_supply, current_ts)
            .unwrap();
        assert_eq!(
            vault.high_water_mark.highest_asset_per_lp_decimal_bits,
            Decimal::ONE.to_bits()
        );
        // TS not updated if not exceeded?
        // Logic: if current > highest { update } else { do nothing }

        // 3. Case: Current > HWM (1.5)
        vault.asset.total_value = 1500;

        vault
            .update_high_water_mark_idempotent(lp_supply, current_ts + 100)
            .unwrap();
        assert_eq!(
            vault.high_water_mark.highest_asset_per_lp_decimal_bits,
            Decimal::from_num(1.5).to_bits()
        );
        assert_eq!(vault.high_water_mark.last_updated_ts, current_ts + 100);
    }

    #[test]
    fn test_zero_lp_supply() {
        let mut vault = Vault::default();
        let current_ts = 100_000;
        vault.asset.total_value = 1000;
        let lp_supply = 0;

        // Should return error due to division by zero when calculating current price
        assert!(vault
            .calibrate_high_water_mark(lp_supply, current_ts)
            .is_err());
    }
}
