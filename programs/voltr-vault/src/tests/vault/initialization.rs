mod tests {
    use crate::state::vault::*;

    #[test]
    fn test_vault_initialization_input_validation() {
        // Test valid fees
        let valid_input = VaultInitializationInput {
            manager_performance_fee: 1000, // 10%
            admin_performance_fee: 500,    // 5%
            manager_management_fee: 100,   // 1%
            admin_management_fee: 100,     // 1%
            redemption_fee: 50,            // 0.5%
            issuance_fee: 25,              // 0.25%
            max_cap: 1_000_000,
            start_at_ts: 1704067200,
            locked_profit_degradation_duration: 86400,
            withdrawal_waiting_period: 3600,
        };
        assert!(valid_input.validate_fees().is_ok());

        // Test performance fees exceed max
        let invalid_performance = VaultInitializationInput {
            manager_performance_fee: 8000, // 80%
            admin_performance_fee: 3000,   // 30% - total 110%
            ..valid_input
        };
        assert!(invalid_performance.validate_fees().is_err());
    }

    #[test]
    fn test_from_implementations() {
        let input = VaultInitializationInput {
            max_cap: 1_000_000,
            start_at_ts: 1704067200,
            manager_performance_fee: 1000,
            admin_performance_fee: 500,
            manager_management_fee: 200,
            admin_management_fee: 100,
            locked_profit_degradation_duration: 86400,
            redemption_fee: 50,
            issuance_fee: 25,
            withdrawal_waiting_period: 3600,
        };

        let vault_config: VaultConfiguration = input.into();
        assert_eq!(vault_config.max_cap, 1_000_000);
        assert_eq!(vault_config.start_at_ts, 1704067200);

        let fee_config: FeeConfiguration = input.into();
        assert_eq!(fee_config.manager_performance_fee, 1000);
        assert_eq!(fee_config.admin_performance_fee, 500);
    }
}
