use anchor_lang::prelude::*;

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct StrategyInitReceipt {
    /// The vault associated with this strategy.
    pub vault: Pubkey,

    /// The strategy address.
    pub strategy: Pubkey,

    /// The adaptor program address.
    pub adaptor_program: Pubkey,

    /// The position value.
    pub position_value: u64,

    /// The last updated timestamp.
    pub last_updated_ts: u64,

    /// A version number (1 byte).
    pub version: u8,

    /// The bump for the strategy init receipt.
    pub bump: u8,

    /// The bump for the vault strategy auth.
    pub vault_strategy_auth_bump: u8,

    /// 6 bytes of padding to align future 8-byte fields on 8-byte boundaries.
    pub _padding0: [u8; 5],

    /// Reserved space for future fields
    pub _reserved: [u8; 64],
}

impl StrategyInitReceipt {
    pub fn set_position_value(&mut self, value: u64, current_ts: u64) -> Result<()> {
        self.position_value = value;
        self.last_updated_ts = current_ts;

        Ok(())
    }

    pub fn init(
        &mut self,
        vault: Pubkey,
        strategy: Pubkey,
        adaptor_program: Pubkey,
        bump: u8,
        vault_strategy_auth_bump: u8,
        current_ts: u64,
    ) -> Result<()> {
        self.vault = vault;
        self.strategy = strategy;
        self.adaptor_program = adaptor_program;
        self.bump = bump;
        self.vault_strategy_auth_bump = vault_strategy_auth_bump;
        self.version = 1;
        self.position_value = 0;
        self.last_updated_ts = current_ts;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::state::StrategyInitReceipt;
    use anchor_lang::prelude::*;

    fn create_test_strategy_init_receipt() -> StrategyInitReceipt {
        StrategyInitReceipt {
            vault: Pubkey::default(),
            strategy: Pubkey::default(),
            adaptor_program: Pubkey::default(),
            position_value: 0,
            last_updated_ts: 0,
            version: 0,
            bump: 0,
            vault_strategy_auth_bump: 0,
            _padding0: [0u8; 5],
            _reserved: [0u8; 64],
        }
    }

    #[test]
    fn test_strategy_init_receipt_initialization() {
        let mut strategy_init_receipt = create_test_strategy_init_receipt();

        let vault = Pubkey::new_unique();
        let strategy = Pubkey::new_unique();
        let adaptor_program = Pubkey::new_unique();
        let bump = 255;
        let vault_strategy_auth_bump = 255;
        let current_ts = 1725308400;

        strategy_init_receipt
            .init(
                vault,
                strategy,
                adaptor_program,
                bump,
                vault_strategy_auth_bump,
                current_ts,
            )
            .unwrap();

        assert_eq!(strategy_init_receipt.vault, vault);
        assert_eq!(strategy_init_receipt.strategy, strategy);
        assert_eq!(strategy_init_receipt.adaptor_program, adaptor_program);
        assert_eq!(strategy_init_receipt.bump, bump);
        assert_eq!(
            strategy_init_receipt.vault_strategy_auth_bump,
            vault_strategy_auth_bump
        );
        assert_eq!(strategy_init_receipt.position_value, 0);
        assert_eq!(strategy_init_receipt.last_updated_ts, current_ts);
        assert_eq!(strategy_init_receipt.version, 1);
    }

    #[test]
    fn test_strategy_init_receipt_set_position_value() {
        let mut strategy_init_receipt = create_test_strategy_init_receipt();

        let value = 1000;
        let current_ts = 1725308400;

        strategy_init_receipt
            .set_position_value(value, current_ts)
            .unwrap();

        assert_eq!(strategy_init_receipt.position_value, value);
        assert_eq!(strategy_init_receipt.last_updated_ts, current_ts);
    }
}
