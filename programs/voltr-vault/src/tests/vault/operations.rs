mod tests {
    use crate::state::vault::*;

    #[test]
    fn test_vault_value_operations() {
        let mut vault = Vault::default();
        vault.asset.total_value = 1000;

        // Test increment
        vault.increment_total_value(500).unwrap();
        assert_eq!(vault.asset.total_value, 1500);

        // Test decrement
        vault.decrement_total_value(300).unwrap();
        assert_eq!(vault.asset.total_value, 1200);
    }

    #[test]
    fn test_vault_value_overflow_underflow() {
        let mut vault = Vault::default();
        vault.asset.total_value = 1000;

        // Test decrement more than available (underflow)
        let result = vault.decrement_total_value(1500);
        assert!(result.is_err());

        // Test increment causing overflow
        vault.asset.total_value = u64::MAX - 100;
        let result = vault.increment_total_value(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_unlocked_asset_calculation() {
        let mut vault = Vault::default();
        vault.asset.total_value = 1000;
        vault.locked_profit_state.last_updated_locked_profit = 300;
        vault.locked_profit_state.last_report = 0;
        vault.vault_configuration.locked_profit_degradation_duration = 100;

        // At time 0, all profit is locked
        assert_eq!(vault.get_unlocked_asset_value(0).unwrap(), 700);

        // At time 50, half profit is unlocked
        assert_eq!(vault.get_unlocked_asset_value(50).unwrap(), 850);

        // At time 100+, all profit is unlocked
        assert_eq!(vault.get_unlocked_asset_value(100).unwrap(), 1000);
    }
}
