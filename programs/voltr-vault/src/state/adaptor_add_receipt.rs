use anchor_lang::prelude::*;

use crate::error::VaultError;

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct AdaptorAddReceipt {
    /// The vault associated with this strategy.
    pub vault: Pubkey,

    /// The adapter program address.
    pub adaptor_program: Pubkey,

    /// A version number (1 byte).
    pub version: u8,

    /// The bump for the adaptor add receipt.
    pub bump: u8,

    /// 7 bytes of padding to align future 8-byte fields on 8-byte boundaries.
    pub _padding0: [u8; 7],

    /// The epoch at which the strategy was last updated.
    pub last_updated_epoch: u64,

    /// Reserved space for future fields
    pub _reserved: [u8; 56],
}

impl AdaptorAddReceipt {
    pub fn init(
        &mut self,
        vault: Pubkey,
        adaptor_program: Pubkey,
        version: u8,
        bump: u8,
        vault_total_value: u64,
        current_epoch: u64,
    ) -> Result<()> {
        self.vault = vault;
        self.adaptor_program = adaptor_program;
        self.version = version;
        self.bump = bump;
        // if vault total value is 0, it is ok to enable adaptor immediately
        self.last_updated_epoch = if vault_total_value > 0 {
            current_epoch
        } else {
            0
        };

        Ok(())
    }

    pub fn check_adaptor_validity(&self, current_epoch: u64) -> Result<()> {
        let valid_from_epoch = self.last_updated_epoch + 2;
        let invalid_epoch = current_epoch < valid_from_epoch;
        if invalid_epoch {
            return Err(VaultError::AdaptorEpochInvalid.into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::state::AdaptorAddReceipt;
    use anchor_lang::prelude::*;

    fn create_test_adaptor_add_receipt() -> AdaptorAddReceipt {
        AdaptorAddReceipt {
            vault: Pubkey::default(),
            adaptor_program: Pubkey::default(),
            version: 0,
            bump: 0,
            _padding0: [0u8; 7],
            last_updated_epoch: 0,
            _reserved: [0u8; 56],
        }
    }

    #[test]
    fn test_adaptor_add_receipt_initialization() {
        let mut adaptor_add_receipt_postive_total_value = create_test_adaptor_add_receipt();

        let vault = Pubkey::new_unique();
        let adaptor_program = Pubkey::new_unique();
        let version = 2;
        let bump = 255;
        let vault_total_value = 1000;
        let current_epoch = 1;

        adaptor_add_receipt_postive_total_value
            .init(
                vault,
                adaptor_program,
                version,
                bump,
                vault_total_value,
                current_epoch,
            )
            .unwrap();

        assert_eq!(adaptor_add_receipt_postive_total_value.vault, vault);
        assert_eq!(
            adaptor_add_receipt_postive_total_value.adaptor_program,
            adaptor_program
        );
        assert_eq!(adaptor_add_receipt_postive_total_value.version, version);
        assert_eq!(adaptor_add_receipt_postive_total_value.bump, bump);
        assert_eq!(
            adaptor_add_receipt_postive_total_value.last_updated_epoch,
            1
        );

        let mut adaptor_add_receipt_zero_total_value = create_test_adaptor_add_receipt();

        adaptor_add_receipt_zero_total_value
            .init(vault, adaptor_program, version, bump, 0, current_epoch)
            .unwrap();

        assert_eq!(adaptor_add_receipt_zero_total_value.vault, vault);
        assert_eq!(
            adaptor_add_receipt_zero_total_value.adaptor_program,
            adaptor_program
        );
        assert_eq!(adaptor_add_receipt_zero_total_value.version, version);
        assert_eq!(adaptor_add_receipt_zero_total_value.bump, bump);
        assert_eq!(adaptor_add_receipt_zero_total_value.last_updated_epoch, 0);
    }

    #[test]
    fn test_adaptor_validity() {
        let mut adaptor_add_receipt = create_test_adaptor_add_receipt();

        let vault = Pubkey::new_unique();
        let adaptor_program = Pubkey::new_unique();
        let version = 2;
        let bump = 255;
        let vault_total_value = 1000;
        let current_epoch = 1;

        adaptor_add_receipt
            .init(
                vault,
                adaptor_program,
                version,
                bump,
                vault_total_value,
                current_epoch,
            )
            .unwrap();

        let current_epoch_valid = adaptor_add_receipt.check_adaptor_validity(current_epoch);

        assert_eq!(current_epoch_valid.is_err(), true);

        let future_epoch_valid = adaptor_add_receipt.check_adaptor_validity(current_epoch + 2);

        assert_eq!(future_epoch_valid.is_ok(), true);
    }
}
