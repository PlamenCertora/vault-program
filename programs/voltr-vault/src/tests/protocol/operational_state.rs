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
    fn test_operational_state_basic_operations() {
        let protocol = Protocol {
            operational_state: OperationalState::CREATE_VAULT.bits(),
            ..create_test_protocol()
        };

        // Should allow CREATE_VAULT
        assert!(protocol
            .check_operation_state(OperationalState::CREATE_VAULT)
            .is_ok());

        // Should not allow other operations
        assert!(protocol
            .check_operation_state(OperationalState::DEPOSIT_VAULT)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::WITHDRAW_VAULT)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::HARVEST_FEE)
            .is_err());
    }

    #[test]
    fn test_operational_state_combined_operations() {
        let combined_state = OperationalState::CREATE_VAULT
            | OperationalState::DEPOSIT_VAULT
            | OperationalState::WITHDRAW_VAULT;

        let protocol = Protocol {
            operational_state: combined_state.bits(),
            ..create_test_protocol()
        };

        // Should allow all included operations
        assert!(protocol
            .check_operation_state(OperationalState::CREATE_VAULT)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::DEPOSIT_VAULT)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::WITHDRAW_VAULT)
            .is_ok());

        // Should not allow excluded operations
        assert!(protocol
            .check_operation_state(OperationalState::HARVEST_FEE)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::ADD_ADAPTOR)
            .is_err());
    }

    #[test]
    fn test_operational_state_no_operations() {
        let protocol = Protocol {
            operational_state: 0, // No operations allowed
            ..create_test_protocol()
        };

        // Should not allow any operations
        assert!(protocol
            .check_operation_state(OperationalState::CREATE_VAULT)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::DEPOSIT_VAULT)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::WITHDRAW_VAULT)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::HARVEST_FEE)
            .is_err());

        // Verify the error type
        let result = protocol.check_operation_state(OperationalState::CREATE_VAULT);
        match result {
            Err(error) => {
                // Check if it's the expected VaultError::OperationNotAllowed
                assert!(error.to_string().contains("Operation not allowed"));
            }
            Ok(_) => panic!("Expected OperationNotAllowed error"),
        }
    }

    #[test]
    fn test_operational_state_bitflag_properties() {
        // Test individual flag values are powers of 2
        assert_eq!(OperationalState::CREATE_VAULT.bits(), 1);
        assert_eq!(OperationalState::DEPOSIT_VAULT.bits(), 2);
        assert_eq!(OperationalState::WITHDRAW_VAULT.bits(), 4);
        assert_eq!(OperationalState::ADD_ADAPTOR.bits(), 8);
        assert_eq!(OperationalState::INIT_STRATEGY.bits(), 16);
        assert_eq!(OperationalState::DEPOSIT_STRATEGY.bits(), 32);
        assert_eq!(OperationalState::WITHDRAW_STRATEGY.bits(), 64);
        assert_eq!(OperationalState::REMOVE_ADAPTOR.bits(), 128);
        assert_eq!(OperationalState::HARVEST_FEE.bits(), 256);
        assert_eq!(OperationalState::INIT_DIRECT_WITHDRAW.bits(), 512);
        assert_eq!(OperationalState::DIRECT_WITHDRAW.bits(), 1024);
        assert_eq!(OperationalState::UPDATE_VAULT.bits(), 2048);
        assert_eq!(OperationalState::CLOSE_STRATEGY.bits(), 4096);

        // Test flag combinations don't interfere
        let combo = OperationalState::CREATE_VAULT | OperationalState::DEPOSIT_VAULT;
        assert_eq!(combo.bits(), 3); // 1 + 2 = 3
    }

    #[test]
    fn test_operational_state_contains_logic() {
        let multi_ops = OperationalState::CREATE_VAULT
            | OperationalState::DEPOSIT_VAULT
            | OperationalState::HARVEST_FEE;

        // Test contains() method behavior
        assert!(multi_ops.contains(OperationalState::CREATE_VAULT));
        assert!(multi_ops.contains(OperationalState::DEPOSIT_VAULT));
        assert!(multi_ops.contains(OperationalState::HARVEST_FEE));
        assert!(!multi_ops.contains(OperationalState::WITHDRAW_VAULT));

        // Test empty state
        let empty_ops = OperationalState::from_bits_truncate(0);
        assert!(!empty_ops.contains(OperationalState::CREATE_VAULT));
    }

    #[test]
    fn test_operational_state_from_bits_truncate() {
        // Test valid combinations
        let valid_combo =
            OperationalState::CREATE_VAULT.bits() | OperationalState::DEPOSIT_VAULT.bits();
        let state = OperationalState::from_bits_truncate(valid_combo);
        assert!(state.contains(OperationalState::CREATE_VAULT));
        assert!(state.contains(OperationalState::DEPOSIT_VAULT));

        // Test with invalid/unknown bits (should truncate unknown bits)
        let invalid_bits = 0x8000; // Bit that doesn't correspond to any defined operation
        let state = OperationalState::from_bits_truncate(invalid_bits);
        assert_eq!(state.bits(), 0); // Should truncate unknown bits
    }

    #[test]
    fn test_protocol_realistic_scenarios() {
        let mut protocol = Protocol::default();
        let admin = Pubkey::new_unique();

        // Scenario 1: Protocol launch - only basic operations
        let launch_ops = OperationalState::CREATE_VAULT | OperationalState::ADD_ADAPTOR;
        protocol.init(admin, launch_ops.bits(), 200).unwrap();

        assert!(protocol
            .check_operation_state(OperationalState::CREATE_VAULT)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::ADD_ADAPTOR)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::DEPOSIT_VAULT)
            .is_err());

        // Scenario 2: Production ready - enable user operations
        let production_ops = launch_ops
            | OperationalState::DEPOSIT_VAULT
            | OperationalState::WITHDRAW_VAULT
            | OperationalState::INIT_STRATEGY
            | OperationalState::DEPOSIT_STRATEGY
            | OperationalState::WITHDRAW_STRATEGY;

        protocol.update(admin, production_ops.bits()).unwrap();

        assert!(protocol
            .check_operation_state(OperationalState::DEPOSIT_VAULT)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::WITHDRAW_VAULT)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::HARVEST_FEE)
            .is_err());

        // Scenario 3: Emergency pause - disable user operations, keep admin operations
        let emergency_ops = OperationalState::UPDATE_VAULT | OperationalState::HARVEST_FEE;
        protocol.update(admin, emergency_ops.bits()).unwrap();

        assert!(protocol
            .check_operation_state(OperationalState::UPDATE_VAULT)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::HARVEST_FEE)
            .is_ok());
        assert!(protocol
            .check_operation_state(OperationalState::DEPOSIT_VAULT)
            .is_err());
        assert!(protocol
            .check_operation_state(OperationalState::WITHDRAW_VAULT)
            .is_err());
    }
}
