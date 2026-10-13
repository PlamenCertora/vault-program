mod tests {
    use crate::state::vault::*;
    use crate::utils::constants::ONE_YEAR_U64;
    use crate::Protocol;

    #[test]
    fn test_concurrent_fee_scenarios() {
        let mut vault = Vault::default();
        vault.asset.total_value = 1_000_000;
        vault.fee_configuration.manager_management_fee = 200; // 2%
        vault.fee_configuration.admin_management_fee = 100; // 1%
        vault.fee_configuration.protocol_management_fee = 100; // 1%
        vault.fee_configuration.manager_performance_fee = 1000; // 10%
        vault.fee_configuration.admin_performance_fee = 500; // 5%
        vault.fee_configuration.protocol_performance_fee = 500; // 5%
        vault.fee_update.last_management_fee_update_ts = 0;

        let current_time = ONE_YEAR_U64;
        let lp_supply = 1_000_000;

        // Apply management fee first
        vault
            .handle_management_fee(current_time, vault.asset.total_value, lp_supply)
            .unwrap();
        let fees_after_mgmt = vault.get_total_accumulated_lp_fees().unwrap();

        // Then apply performance fee with profit
        vault
            .high_water_mark
            .calibrate_high_water_mark(
                vault.asset.total_value,
                lp_supply + fees_after_mgmt,
                current_time,
            )
            .unwrap();
        vault
            .handle_performance_fee_and_update_vault(true, 200_000, 9, lp_supply, 9, current_time)
            .unwrap();

        // Verify both fees were applied correctly
        assert!(vault.fee_state.accumulated_lp_admin_fees > 0);
        assert!(vault.fee_state.accumulated_lp_manager_fees > 0);
        assert!(vault.fee_state.accumulated_lp_protocol_fees > 0);
    }

    #[test]
    fn test_full_vault_lifecycle() {
        let mut vault = Vault::default();

        // Initialize vault
        vault.asset.total_value = 1_000_000;
        vault.fee_configuration = FeeConfiguration {
            manager_performance_fee: 1000,
            admin_performance_fee: 500,
            manager_management_fee: 200,
            admin_management_fee: 100,
            redemption_fee: 50,
            issuance_fee: 25,
            protocol_performance_fee: 0,
            protocol_management_fee: 0,
            reserved: [0u8; 32],
        };

        let protocol = Protocol {
            ..Default::default()
        };

        // Simulate vault operations over time
        let lp_supply = 1_000_000;
        let year_1 = ONE_YEAR_U64;

        // Year 1: Management fees
        vault
            .handle_management_fee(year_1, vault.asset.total_value, lp_supply)
            .unwrap();

        // Profit event
        vault
            .high_water_mark
            .calibrate_high_water_mark(vault.asset.total_value, lp_supply, year_1)
            .unwrap();
        vault
            .handle_performance_fee_and_update_vault(true, 200_000, 9, lp_supply, 9, year_1)
            .unwrap();

        // Year 2: More operations
        let year_2 = year_1 + ONE_YEAR_U64;
        let new_lp_supply = lp_supply + vault.get_total_accumulated_lp_fees().unwrap();

        vault
            .handle_management_fee(year_2, vault.asset.total_value, new_lp_supply)
            .unwrap();

        // Verify final state
        assert!(vault.asset.total_value > 1_000_000);
        assert!(vault.get_total_accumulated_lp_fees().unwrap() > 0);
        assert!(vault.high_water_mark.highest_asset_per_lp_decimal_bits > 0);
    }
}
