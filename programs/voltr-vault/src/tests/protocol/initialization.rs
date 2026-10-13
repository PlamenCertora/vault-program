mod tests {
    use crate::state::protocol::*;
    use anchor_lang::prelude::*;

    fn create_test_protocol() -> Protocol {
        Protocol {
            admin: Pubkey::new_unique(),
            operational_state: OperationalState::CREATE_VAULT.bits(),
            _padding1: [0u8; 2],
            bump: 255,
            _padding0: [0u8; 1],
            _reserved: [0u8; 64],
        }
    }

    #[test]
    fn test_protocol_initialization() {
        let mut protocol = Protocol::default();
        let admin = Pubkey::new_unique();
        let operational_state = OperationalState::CREATE_VAULT | OperationalState::DEPOSIT_VAULT;
        let bump = 200;

        protocol
            .init(admin, operational_state.bits(), bump)
            .unwrap();

        assert_eq!(protocol.admin, admin);
        assert_eq!(protocol.operational_state, operational_state.bits());
        assert_eq!(protocol.bump, bump);
    }

    #[test]
    fn test_protocol_update() {
        let mut protocol = create_test_protocol();
        let original_bump = protocol.bump;

        let new_admin = Pubkey::new_unique();
        let new_operational_state =
            OperationalState::WITHDRAW_VAULT | OperationalState::HARVEST_FEE;

        protocol
            .update(new_admin, new_operational_state.bits())
            .unwrap();

        assert_eq!(protocol.admin, new_admin);
        assert_eq!(protocol.operational_state, new_operational_state.bits());
        // Bump should remain unchanged during update
        assert_eq!(protocol.bump, original_bump);
    }
}
