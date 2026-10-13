mod tests {
    use crate::state::vault::*;
    use crate::utils::constants::ONE_YEAR_U64;

    #[test]
    fn test_handle_management_fee_normal_case() {
        let mut vault = Vault::default();
        setup_vault_for_management_fee(&mut vault);

        let one_year_later = vault.fee_update.last_management_fee_update_ts + ONE_YEAR_U64;
        let total_lp_supply = 1_000_000_000;

        vault
            .handle_management_fee(one_year_later, vault.asset.total_value, total_lp_supply)
            .unwrap();

        // Verify fees were calculated and accumulated
        assert!(vault.fee_state.accumulated_lp_admin_fees > 0);
        assert!(vault.fee_state.accumulated_lp_manager_fees > 0);
        assert_eq!(
            vault.fee_update.last_management_fee_update_ts,
            one_year_later
        );
    }

    #[test]
    fn test_management_fee_edge_cases() {
        let mut vault = Vault::default();
        vault.fee_configuration.manager_management_fee = 200;
        vault.fee_configuration.admin_management_fee = 100;

        // Test with zero asset value
        vault.asset.total_value = 0;
        vault.fee_update.last_management_fee_update_ts = 1704067200;
        assert!(vault.handle_management_fee(1704067300, 0, 1000).is_ok());
        assert_eq!(vault.fee_state.accumulated_lp_admin_fees, 0);

        // Test with zero management fee
        vault.asset.total_value = 1_000_000;
        vault.fee_configuration.manager_management_fee = 0;
        vault.fee_configuration.admin_management_fee = 0;
        assert!(vault
            .handle_management_fee(1704067300, 1_000_000, 1000)
            .is_ok());
        assert_eq!(vault.fee_state.accumulated_lp_admin_fees, 0);
    }

    fn setup_vault_for_management_fee(vault: &mut Vault) {
        vault.asset.total_value = 1_000_000_000;
        vault.fee_configuration.manager_management_fee = 100;
        vault.fee_configuration.admin_management_fee = 100;
        vault.fee_update.last_management_fee_update_ts = 1704067200;
    }
}
