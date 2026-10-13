use anchor_lang::prelude::*;

use crate::utils::constants::WITHDRAW_ANCHOR_DISCRIMINATOR;

#[account]
pub struct DirectWithdrawInitReceipt {
    /// The vault associated with this strategy.
    pub vault: Pubkey,

    /// The strategy address.
    pub strategy: Pubkey,

    /// The position value.
    pub adaptor_program: Pubkey,

    /// The instruction discriminator.
    pub instruction_discriminator: Vec<u8>,

    /// The additional arguments.
    pub additional_args: Option<Vec<u8>>,

    /// Whether the user args are allowed.
    pub allow_user_args: bool,

    /// A version number (1 byte).
    pub version: u8,

    /// The bump for the strategy init receipt.
    pub bump: u8,
}

impl DirectWithdrawInitReceipt {
    pub fn len(
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
    ) -> usize {
        let instruction_discriminator_len = 4
            + (match instruction_discriminator {
                Some(discriminator) => discriminator.len(),
                None => WITHDRAW_ANCHOR_DISCRIMINATOR.len(),
            });
        let additional_args_len = 1
            + (match additional_args {
                Some(args) => 4 + args.len(),
                None => 0,
            });
        32 + // vault
            32 + // strategy
            32 + // adaptor_program
            instruction_discriminator_len + // instruction_discriminator
            additional_args_len + // additional_args
            1 + // allow_user_args
            1 + // version
            1 // bump
    }

    pub fn init(
        &mut self,
        vault: Pubkey,
        strategy: Pubkey,
        adaptor_program: Pubkey,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
        allow_user_args: bool,
        version: u8,
        bump: u8,
    ) {
        self.vault = vault;
        self.strategy = strategy;
        self.adaptor_program = adaptor_program;
        self.instruction_discriminator =
            instruction_discriminator.unwrap_or(WITHDRAW_ANCHOR_DISCRIMINATOR.to_vec());
        self.additional_args = additional_args;
        self.allow_user_args = allow_user_args;
        self.version = version;
        self.bump = bump;
    }
}

#[cfg(test)]
mod tests {
    use crate::state::direct_withdraw_init_receipt::*;
    use crate::utils::constants::WITHDRAW_ANCHOR_DISCRIMINATOR;

    #[test]
    fn test_dynamic_length_calculation() {
        // Test 1: No optional fields - should use defaults
        let base_len = DirectWithdrawInitReceipt::len(None, None);
        let expected_base = 32 +
            32 +
            32 + // vault + strategy + adaptor_program
            4 +
            WITHDRAW_ANCHOR_DISCRIMINATOR.len() + // default discriminator
            1 +
            0 + // no additional args
            1 +
            1 +
            1; // allow_user_args + version + bump
        assert_eq!(base_len, expected_base);

        // Test 2: With custom discriminator
        let custom_discriminator = vec![1, 2, 3, 4, 5];
        let custom_len = DirectWithdrawInitReceipt::len(Some(custom_discriminator.clone()), None);
        let expected_custom = 32 +
            32 +
            32 + // vault + strategy + adaptor_program
            4 +
            custom_discriminator.len() + // custom discriminator
            1 +
            0 + // no additional args
            1 +
            1 +
            1; // allow_user_args + version + bump
        assert_eq!(custom_len, expected_custom);

        // Test 3: With additional args
        let additional_args = vec![10, 20, 30];
        let args_len = DirectWithdrawInitReceipt::len(None, Some(additional_args.clone()));
        let expected_args = 32 +
            32 +
            32 + // vault + strategy + adaptor_program
            4 +
            WITHDRAW_ANCHOR_DISCRIMINATOR.len() + // default discriminator
            1 +
            4 +
            additional_args.len() + // additional args with length prefix
            1 +
            1 +
            1; // allow_user_args + version + bump
        assert_eq!(args_len, expected_args);

        // Test 4: Both custom discriminator and additional args
        let both_len = DirectWithdrawInitReceipt::len(
            Some(custom_discriminator.clone()),
            Some(additional_args.clone()),
        );
        let expected_both = 32 +
            32 +
            32 + // vault + strategy + adaptor_program
            4 +
            custom_discriminator.len() + // custom discriminator
            1 +
            4 +
            additional_args.len() + // additional args
            1 +
            1 +
            1; // allow_user_args + version + bump
        assert_eq!(both_len, expected_both);
    }

    #[test]
    fn test_initialization_with_defaults() {
        let mut receipt = DirectWithdrawInitReceipt {
            vault: Pubkey::default(),
            strategy: Pubkey::default(),
            adaptor_program: Pubkey::default(),
            instruction_discriminator: vec![],
            additional_args: None,
            allow_user_args: false,
            version: 0,
            bump: 0,
        };

        let vault = Pubkey::new_unique();
        let strategy = Pubkey::new_unique();
        let adaptor_program = Pubkey::new_unique();

        // Test initialization with None values (should use defaults)
        receipt.init(
            vault,
            strategy,
            adaptor_program,
            None, // Should default to WITHDRAW_ANCHOR_DISCRIMINATOR
            None, // Should remain None
            true,
            1,
            255,
        );

        assert_eq!(receipt.vault, vault);
        assert_eq!(receipt.strategy, strategy);
        assert_eq!(receipt.adaptor_program, adaptor_program);
        assert_eq!(
            receipt.instruction_discriminator,
            WITHDRAW_ANCHOR_DISCRIMINATOR.to_vec()
        );
        assert_eq!(receipt.additional_args, None);
        assert_eq!(receipt.allow_user_args, true);
        assert_eq!(receipt.version, 1);
        assert_eq!(receipt.bump, 255);

        // Test initialization with custom values
        let custom_discriminator = vec![0xaa, 0xbb, 0xcc, 0xdd];
        let custom_args = vec![1, 2, 3, 4, 5];

        receipt.init(
            vault,
            strategy,
            adaptor_program,
            Some(custom_discriminator.clone()),
            Some(custom_args.clone()),
            false,
            2,
            200,
        );

        assert_eq!(receipt.instruction_discriminator, custom_discriminator);
        assert_eq!(receipt.additional_args, Some(custom_args));
        assert_eq!(receipt.allow_user_args, false);
        assert_eq!(receipt.version, 2);
        assert_eq!(receipt.bump, 200);
    }

    #[test]
    fn test_edge_cases_and_boundaries() {
        // Test with empty vectors
        let empty_discriminator = vec![];
        let empty_args = vec![];

        let len_empty = DirectWithdrawInitReceipt::len(
            Some(empty_discriminator.clone()),
            Some(empty_args.clone()),
        );
        let expected_empty = 32 +
            32 +
            32 + // vault + strategy + adaptor_program
            4 +
            0 + // empty discriminator
            1 +
            4 +
            0 + // empty additional args
            1 +
            1 +
            1; // allow_user_args + version + bump
        assert_eq!(len_empty, expected_empty);

        // Test with large vectors
        let large_discriminator = vec![0u8; 1000];
        let large_args = vec![0u8; 2000];

        let len_large = DirectWithdrawInitReceipt::len(
            Some(large_discriminator.clone()),
            Some(large_args.clone()),
        );
        let expected_large = 32 +
            32 +
            32 + // vault + strategy + adaptor_program
            4 +
            1000 + // large discriminator
            1 +
            4 +
            2000 + // large additional args
            1 +
            1 +
            1; // allow_user_args + version + bump
        assert_eq!(len_large, expected_large);

        // Test initialization with edge case values
        let mut receipt = DirectWithdrawInitReceipt {
            vault: Pubkey::default(),
            strategy: Pubkey::default(),
            adaptor_program: Pubkey::default(),
            instruction_discriminator: vec![],
            additional_args: None,
            allow_user_args: false,
            version: 0,
            bump: 0,
        };

        // Test with maximum u8 values
        receipt.init(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Some(large_discriminator),
            Some(large_args),
            true,
            u8::MAX,
            u8::MAX,
        );

        assert_eq!(receipt.version, u8::MAX);
        assert_eq!(receipt.bump, u8::MAX);
        assert_eq!(receipt.allow_user_args, true);

        // Test boolean toggle
        receipt.init(
            receipt.vault,
            receipt.strategy,
            receipt.adaptor_program,
            None,
            None,
            false, // Toggle to false
            0,     // Minimum version
            0,     // Minimum bump
        );

        assert_eq!(receipt.allow_user_args, false);
        assert_eq!(receipt.version, 0);
        assert_eq!(receipt.bump, 0);
    }
}
