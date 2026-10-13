mod tests {
    use crate::state::vault::*;

    #[test]
    fn test_fee_accumulation_and_reset() {
        let mut vault = Vault::default();

        // Test accumulating various fees
        vault.increment_admin_fees(100).unwrap();
        vault.increment_manager_fees(200).unwrap();
        vault.increment_protocol_fees(50).unwrap();

        assert_eq!(vault.fee_state.accumulated_lp_admin_fees, 100);
        assert_eq!(vault.fee_state.accumulated_lp_manager_fees, 200);
        assert_eq!(vault.fee_state.accumulated_lp_protocol_fees, 50);
        assert_eq!(vault.get_total_accumulated_lp_fees().unwrap(), 350);
    }

    #[test]
    fn test_lp_supply_calculation() {
        let mut vault = Vault::default();
        vault.increment_admin_fees(100).unwrap();
        vault.increment_manager_fees(200).unwrap();
        vault.increment_protocol_fees(50).unwrap();

        assert_eq!(vault.get_total_lp_supply_incl_fees(1000).unwrap(), 1350);
    }

    #[test]
    fn test_fee_state_reset() {
        let mut vault = Vault::default();
        vault.increment_admin_fees(100).unwrap();
        vault.increment_manager_fees(200).unwrap();
        vault.increment_protocol_fees(150).unwrap();

        vault.reset_fee_state().unwrap();
        assert_eq!(vault.fee_state.accumulated_lp_admin_fees, 0);
        assert_eq!(vault.fee_state.accumulated_lp_manager_fees, 0);
        assert_eq!(vault.fee_state.accumulated_lp_protocol_fees, 0);
    }
}
