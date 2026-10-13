mod tests {
    use crate::state::vault::*;
    use crate::utils::constants::ONE_YEAR_U64;

    #[test]
    fn test_very_small_amounts() {
        let mut vault = Vault::default();

        // Test with minimal asset values
        vault.asset.total_value = 1;
        vault.fee_configuration.manager_management_fee = 1; // 0.01%
        vault.fee_update.last_management_fee_update_ts = 0;

        // One year later - should handle tiny fee calculations
        let one_year_later = ONE_YEAR_U64;
        assert!(vault.handle_management_fee(one_year_later, 1, 1).is_ok());

        // Test performance fee with minimal profit
        vault.asset.total_value = 1;
        vault.fee_configuration.manager_performance_fee = 1;
        assert!(vault
            .handle_performance_fee_and_update_vault(true, 1, 9, 1, 9, one_year_later)
            .is_ok());
    }

    #[test]
    fn test_fee_configuration_getters() {
        let mut vault = Vault::default();
        vault.fee_configuration.manager_performance_fee = 1000;
        vault.fee_configuration.admin_performance_fee = 500;
        vault.fee_configuration.manager_management_fee = 200;
        vault.fee_configuration.admin_management_fee = 100;

        assert_eq!(
            vault.get_total_fee_configuration_performance_fee().unwrap(),
            1500
        );
        assert_eq!(
            vault.get_total_fee_configuration_management_fee().unwrap(),
            300
        );

        // Test overflow in fee calculation
        vault.fee_configuration.manager_performance_fee = u16::MAX;
        vault.fee_configuration.admin_performance_fee = 1;
        let result = vault.get_total_fee_configuration_performance_fee();
        assert!(result.is_err());
    }
}
