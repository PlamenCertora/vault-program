mod tests {
    use crate::state::vault::*;
    use crate::utils::Decimal;

    #[test]
    fn test_handle_performance_fee_with_profit() {
        let mut vault = Vault::default();
        setup_vault_for_performance_fee(&mut vault);

        let pnl_amount = 200_000_000; // +200 assets
        vault
            .handle_performance_fee_and_update_vault(
                true,
                pnl_amount,
                9,
                1_000_000_000,
                9,
                1704067200,
            )
            .unwrap();

        assert_eq!(vault.asset.total_value, 1_200_000_000);
        assert!(vault.fee_state.accumulated_lp_admin_fees > 0);
        assert!(vault.fee_state.accumulated_lp_manager_fees > 0);
        assert!(vault.fee_state.accumulated_lp_protocol_fees > 0);
    }

    #[test]
    fn test_handle_performance_fee_with_loss() {
        let mut vault = Vault::default();
        setup_vault_for_performance_fee(&mut vault);

        let pnl_amount = 100_000_000; // -100 assets
        vault
            .handle_performance_fee_and_update_vault(
                false,
                pnl_amount,
                9,
                1_000_000_000,
                9,
                1704067200,
            )
            .unwrap();

        assert_eq!(vault.asset.total_value, 900_000_000);
        assert_eq!(vault.fee_state.accumulated_lp_admin_fees, 0);
        assert_eq!(vault.fee_state.accumulated_lp_manager_fees, 0);
    }

    fn setup_vault_for_performance_fee(vault: &mut Vault) {
        vault.asset.total_value = 1_000_000_000;
        vault.high_water_mark.highest_asset_per_lp_decimal_bits = Decimal::ONE.to_bits();
        vault.fee_configuration.manager_performance_fee = 1000;
        vault.fee_configuration.admin_performance_fee = 500;
        vault.fee_configuration.protocol_performance_fee = 500;
    }
}
