use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

use crate::state::InitializeStrategyEvent;
use crate::utils::constants::{INITIALIZE_ANCHOR_DISCRIMINATOR, PROTOCOL_SEED};
use crate::utils::to_account_meta_modify_manager_non_signer_option;
use crate::utils::{
    constants::{ADAPTOR_ADD_RECEIPT_SEED, STRATEGY_INIT_RECEIPT_SEED, VAULT_STRATEGY_AUTH_SEED},
    encode_additional_args,
};
use crate::{AdaptorAddReceipt, OperationalState, Protocol, StrategyInitReceipt, Vault};

#[derive(Accounts)]
pub struct InitializeStrategy<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub manager: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    /// CHECK: vault checked with strategy init and adaptor add receipt seeds
    #[account(
        constraint = vault.load()?.manager == manager.key() || vault.load()?.admin == manager.key()
    )]
    pub vault: AccountLoader<'info, Vault>,

    /// CHECK: strategy checked with strategy init receipt seeds
    pub strategy: AccountInfo<'info>,

    #[account(
        seeds = [ADAPTOR_ADD_RECEIPT_SEED, vault.key().as_ref(), adaptor_program.key().as_ref()],
        bump = adaptor_add_receipt.load()?.bump
    )]
    pub adaptor_add_receipt: AccountLoader<'info, AdaptorAddReceipt>,

    #[account(
        init_if_needed,
        seeds = [STRATEGY_INIT_RECEIPT_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump,
        payer = payer,
        space = std::mem::size_of::<StrategyInitReceipt>() + 8
    )]
    pub strategy_init_receipt: AccountLoader<'info, StrategyInitReceipt>,

    /// CHECK:
    #[account(mut,
        seeds = [
            VAULT_STRATEGY_AUTH_SEED,
            vault.key().as_ref(),
            strategy.key().as_ref()
        ],
        bump
    )]
    pub vault_strategy_auth: AccountInfo<'info>,

    /// CHECK: Do not need as seeds are checked with adaptor_add_receipt seeds
    pub adaptor_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler_initialize_strategy<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InitializeStrategy<'info>>,
    instruction_discriminator: Option<Vec<u8>>,
    additional_args: Option<Vec<u8>>,
) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::INIT_STRATEGY)?;
    ctx.accounts
        .adaptor_add_receipt
        .load()?
        .check_adaptor_validity(Clock::get()?.epoch)?;

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    let is_initialized = ctx.accounts.init_strategy_init_receipt_if_needed(
        ctx.bumps.strategy_init_receipt,
        ctx.bumps.vault_strategy_auth,
        current_ts,
    )?;

    ctx.accounts.invoke_initialize_strategy_cpi(
        &ctx.remaining_accounts,
        ctx.bumps.vault_strategy_auth,
        instruction_discriminator,
        additional_args,
    )?;

    emit!(InitializeStrategyEvent {
        manager: ctx.accounts.manager.key(),
        vault: ctx.accounts.vault.key(),
        vault_strategy_auth: ctx.accounts.vault_strategy_auth.key(),
        strategy: ctx.accounts.strategy.key(),
        strategy_init_receipt: ctx.accounts.strategy_init_receipt.key(),
        adaptor_program: ctx.accounts.adaptor_program.key(),
        initialized_ts: current_ts,
        is_initialized,
    });

    Ok(())
}

impl<'info> InitializeStrategy<'info> {
    fn init_strategy_init_receipt_if_needed(
        &self,
        bump: u8,
        vault_strategy_auth_bump: u8,
        current_ts: u64,
    ) -> Result<bool> {
        // Try to deserialize the vault key and match with self.vault.key()
        let is_initialized = Pubkey::try_from_slice(
            &self.strategy_init_receipt.to_account_info().data.borrow()[8..40],
        )
        .map(|key| key == self.vault.key())
        .unwrap_or(false);

        // Only initialize if the account is fresh (not initialized)
        if !is_initialized {
            let strategy_init_receipt = &mut self.strategy_init_receipt.load_init()?;
            strategy_init_receipt.init(
                self.vault.key(),
                self.strategy.key(),
                self.adaptor_program.key(),
                bump,
                vault_strategy_auth_bump,
                current_ts,
            )?;
        }

        Ok(is_initialized)
    }

    fn invoke_initialize_strategy_cpi(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        vault_strategy_auth_bump: u8,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut data = if let Some(discriminator) = instruction_discriminator {
            discriminator
        } else {
            INITIALIZE_ANCHOR_DISCRIMINATOR.to_vec()
        };
        encode_additional_args(&mut data, &additional_args);

        let cpi_accounts = &mut vec![
            AccountMeta::new(self.payer.key(), true),
            AccountMeta::new(self.vault_strategy_auth.key(), true),
            AccountMeta::new_readonly(self.strategy.key(), false),
            AccountMeta::new_readonly(self.system_program.key(), false),
        ];

        let manager_key_option = if self.payer.key().ne(&self.manager.key()) {
            Some(self.manager.key())
        } else {
            None
        };

        cpi_accounts.extend(remaining_accounts.iter().map(|acc| {
            to_account_meta_modify_manager_non_signer_option(acc, manager_key_option.as_ref())
        }));

        let cpi_instruction = Instruction {
            program_id: self.adaptor_program.key(),
            accounts: cpi_accounts.to_vec(),
            data,
        };

        let account_infos = &mut vec![
            self.payer.to_account_info(),
            self.vault_strategy_auth.to_account_info(),
            self.strategy.to_account_info(),
            self.system_program.to_account_info(),
        ];

        account_infos.extend(remaining_accounts.iter().map(|acc| acc.to_account_info()));

        solana_invoke::invoke_signed(
            &cpi_instruction,
            &account_infos,
            &[&[
                VAULT_STRATEGY_AUTH_SEED,
                self.vault.key().as_ref(),
                self.strategy.key().as_ref(),
                &[vault_strategy_auth_bump],
            ]],
        )?;
        Ok(())
    }
}
