use anchor_lang::prelude::*;

use crate::error::VaultError;

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct Protocol {
    /// The admin of the protocol.
    pub admin: Pubkey,

    /// The operational state of the protocol.
    pub operational_state: u16,

    /// previously used for fee, now unused
    pub _padding1: [u8; 2],

    /// The bump for the protocol.
    pub bump: u8,

    /// 1 byte of padding to align future 8-byte fields on 8-byte boundaries.
    pub _padding0: [u8; 1],

    /// Reserved space for future fields
    pub _reserved: [u8; 64],
}

impl Default for Protocol {
    fn default() -> Self {
        Self {
            admin: Pubkey::default(),
            operational_state: 0,
            _padding1: [0u8; 2],
            bump: 0,
            _padding0: [0u8; 1],
            _reserved: [0u8; 64],
        }
    }
}

impl Protocol {
    pub fn init(&mut self, admin: Pubkey, operational_state: u16, bump: u8) -> Result<()> {
        self.update(admin, operational_state)?;
        self.bump = bump;

        Ok(())
    }

    pub fn update(&mut self, admin: Pubkey, operational_state: u16) -> Result<()> {
        self.admin = admin;
        self.operational_state = operational_state;

        Ok(())
    }

    pub fn check_operation_state(&self, operation: OperationalState) -> Result<()> {
        let current_flags = OperationalState::from_bits_truncate(self.operational_state);
        if !current_flags.contains(operation) {
            return Err(VaultError::OperationNotAllowed.into());
        }

        Ok(())
    }
}

bitflags::bitflags! {
    pub struct OperationalState: u16 {
        const CREATE_VAULT = 1 << 0;
        const DEPOSIT_VAULT = 1 << 1;
        const WITHDRAW_VAULT = 1 << 2;
        const ADD_ADAPTOR = 1 << 3;
        const INIT_STRATEGY = 1 << 4;
        const DEPOSIT_STRATEGY = 1 << 5;
        const WITHDRAW_STRATEGY = 1 << 6;
        const REMOVE_ADAPTOR = 1 << 7;
        const HARVEST_FEE = 1 << 8;
        const INIT_DIRECT_WITHDRAW = 1 << 9;
        const DIRECT_WITHDRAW = 1 << 10;
        const UPDATE_VAULT = 1 << 11;
        const CLOSE_STRATEGY = 1 << 12;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_implementations() {
        let protocol = Protocol::default();
        assert_eq!(protocol.admin, Pubkey::default());
        assert_eq!(protocol.operational_state, 0);
        assert_eq!(protocol.bump, 0);
    }
}
