use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    state::WithdrawStrategyEvent,
    utils::{
        calc_withdraw_pnl,
        constants::{
            ADAPTOR_ADD_RECEIPT_SEED, PROTOCOL_SEED, STRATEGY_INIT_RECEIPT_SEED,
            VAULT_ASSET_IDLE_AUTH_SEED, VAULT_LP_MINT_SEED, VAULT_STRATEGY_AUTH_SEED,
            WITHDRAW_ANCHOR_DISCRIMINATOR,
        },
        encode_additional_args, get_return_u64, to_account_meta_modify_manager_non_signer_option,
    },
    AdaptorAddReceipt, OperationalState, Protocol, StrategyInitReceipt, Vault,
};

#[derive(Accounts)]
pub struct WithdrawStrategy<'info> {
    /// CHECK:
    #[account()]
    pub manager: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    /// CHECK: vault checked with strategy init receipt seeds
    #[account(
        mut,
        constraint = vault.load()?.asset.mint == vault_asset_mint.key(),
        constraint = vault.load()?.manager == manager.key() || vault.load()?.admin == manager.key()
    )]
    pub vault: AccountLoader<'info, Vault>,

    #[account(
        seeds = [ADAPTOR_ADD_RECEIPT_SEED, vault.key().as_ref(), adaptor_program.key().as_ref()],
        bump = adaptor_add_receipt.load()?.bump
    )]
    pub adaptor_add_receipt: AccountLoader<'info, AdaptorAddReceipt>,

    #[account(
        mut,
        seeds = [STRATEGY_INIT_RECEIPT_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump = strategy_init_receipt.load()?.bump
    )]
    pub strategy_init_receipt: AccountLoader<'info, StrategyInitReceipt>,

    /// CHECK: strategy checked with strategy init receipt seeds
    pub strategy: AccountInfo<'info>,

    /// CHECK: adaptor program checked with adaptor add receipt seeds
    #[account(address = strategy_init_receipt.load()?.adaptor_program)]
    pub adaptor_program: AccountInfo<'info>,

    /// CHECK:
    #[account(mut, seeds = [VAULT_ASSET_IDLE_AUTH_SEED, vault.key().as_ref()], bump = vault.load()?.asset.idle_ata_auth_bump)]
    pub vault_asset_idle_auth: AccountInfo<'info>,

    /// CHECK:
    #[account(
        mut,
        seeds = [VAULT_STRATEGY_AUTH_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump = strategy_init_receipt.load()?.vault_strategy_auth_bump
    )]
    pub vault_strategy_auth: AccountInfo<'info>,

    #[account(mut, mint::token_program = asset_token_program)]
    pub vault_asset_mint: InterfaceAccount<'info, Mint>,

    /// CHECK:
    #[account(
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,

    /// The vault's associated token account for asset.
    #[account(
        mut,
        associated_token::mint = vault_asset_mint,
        associated_token::authority = vault_asset_idle_auth,
        associated_token::token_program = asset_token_program,
    )]
    pub vault_asset_idle_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(
            mut,
            associated_token::mint = vault_asset_mint,
            associated_token::authority = vault_strategy_auth,
            associated_token::token_program = asset_token_program,
        )]
    pub vault_strategy_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub asset_token_program: Interface<'info, TokenInterface>,
}

pub fn handler_withdraw_strategy<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, WithdrawStrategy<'info>>,
    amount: u64,
    instruction_discriminator: Option<Vec<u8>>,
    additional_args: Option<Vec<u8>>,
) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::WITHDRAW_STRATEGY)?;
    ctx.accounts
        .adaptor_add_receipt
        .load()?
        .check_adaptor_validity(Clock::get()?.epoch)?;

    let vault_strategy_auth_bump = ctx
        .accounts
        .strategy_init_receipt
        .load()?
        .vault_strategy_auth_bump;

    let post_invoke_strategy_position_value = ctx
        .accounts
        .invoke_withdraw_strategy_cpi_get_position_value(
            ctx.remaining_accounts,
            amount,
            vault_strategy_auth_bump,
            instruction_discriminator,
            additional_args,
        )?;

    let post_invoke_strategy_ata_amount = ctx.accounts.get_post_invoke_strategy_ata_amount()?;

    if post_invoke_strategy_ata_amount > 0 {
        ctx.accounts
            .transfer_strategy_to_idle_ata(post_invoke_strategy_ata_amount)?;
    }

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    let strategy_init_receipt = &mut ctx.accounts.strategy_init_receipt.load_mut()?;

    let pre_invoked_strategy_position_value = strategy_init_receipt.position_value;

    strategy_init_receipt.set_position_value(post_invoke_strategy_position_value, current_ts)?;

    let (is_profit, pnl_amount) = calc_withdraw_pnl(
        pre_invoked_strategy_position_value,
        post_invoke_strategy_position_value,
        post_invoke_strategy_ata_amount,
    )?;

    let vault = &mut ctx.accounts.vault.load_mut()?;

    let vault_asset_idle_ata_amount_before = ctx.accounts.vault_asset_idle_ata.amount;
    ctx.accounts.vault_asset_idle_ata.reload()?;
    let vault_asset_idle_ata_amount_after = ctx.accounts.vault_asset_idle_ata.amount;
    let vault_asset_total_value_before = vault.get_total_asset_value();
    let vault_lp_supply_incl_fees_before =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;
    let vault_highest_asset_per_lp_decimal_bits_before = vault
        .high_water_mark
        .get_highest_asset_per_lp_decimal_bits()?;

    // Call shared function for fee handling
    vault.handle_performance_fee_and_update_vault(
        is_profit,
        pnl_amount,
        ctx.accounts.vault_asset_mint.decimals,
        ctx.accounts.vault_lp_mint.supply,
        ctx.accounts.vault_lp_mint.decimals,
        current_ts,
    )?;
    vault.update_high_water_mark_idempotent(ctx.accounts.vault_lp_mint.supply, current_ts)?;
    vault.update_last_updated_ts(current_ts)?;

    emit!(WithdrawStrategyEvent {
        manager: ctx.accounts.manager.key(),
        vault: ctx.accounts.vault.key(),
        strategy: ctx.accounts.strategy.key(),
        strategy_init_receipt: ctx.accounts.strategy_init_receipt.key(),
        adaptor_program: ctx.accounts.adaptor_program.key(),
        vault_asset_mint: ctx.accounts.vault_asset_mint.key(),
        vault_amount_asset_withdrawn: amount,
        vault_asset_total_value_before,
        vault_asset_total_value_after: vault.get_total_asset_value(),
        vault_lp_supply_incl_fees_before,
        vault_lp_supply_incl_fees_after: vault
            .get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after: vault
            .high_water_mark
            .get_highest_asset_per_lp_decimal_bits()?,
        vault_asset_idle_ata_amount_before,
        vault_asset_idle_ata_amount_after,
        strategy_position_value_before: pre_invoked_strategy_position_value,
        strategy_position_value_after: post_invoke_strategy_position_value,
        withdrawn_ts: vault.last_updated_ts,
    });
    Ok(())
}

impl<'info> WithdrawStrategy<'info> {
    /// Transfers assets from vault strategy ATA to vault idle ATA
    pub fn transfer_strategy_to_idle_ata(&self, amount: u64) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.vault_strategy_asset_ata.to_account_info(),
                    to: self.vault_asset_idle_ata.to_account_info(),
                    authority: self.vault_strategy_auth.to_account_info(),
                    mint: self.vault_asset_mint.to_account_info(),
                },
                &[&[
                    VAULT_STRATEGY_AUTH_SEED,
                    self.vault.key().as_ref(),
                    self.strategy.key().as_ref(),
                    &[self.strategy_init_receipt.load()?.vault_strategy_auth_bump],
                ]],
            ),
            amount,
            self.vault_asset_mint.decimals,
        )
    }

    /// Builds and invokes the withdraw strategy CPI instruction
    pub fn invoke_withdraw_strategy_cpi_get_position_value(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        amount: u64,
        vault_strategy_auth_bump: u8,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
    ) -> Result<u64> {
        let mut data = if let Some(discriminator) = instruction_discriminator {
            discriminator
        } else {
            WITHDRAW_ANCHOR_DISCRIMINATOR.to_vec()
        };
        data.extend_from_slice(&amount.to_le_bytes());
        encode_additional_args(&mut data, &additional_args);

        let mut cpi_accounts = vec![
            AccountMeta::new(self.vault_strategy_auth.key(), true),
            AccountMeta::new_readonly(self.strategy.key(), false),
            AccountMeta::new(self.vault_asset_mint.key(), false),
            AccountMeta::new(self.vault_strategy_asset_ata.key(), false),
            AccountMeta::new_readonly(self.asset_token_program.key(), false),
        ];
        cpi_accounts.extend(remaining_accounts.iter().map(|acc| {
            to_account_meta_modify_manager_non_signer_option(acc, Some(&self.manager.key()))
        }));

        let instruction = Instruction {
            program_id: self.adaptor_program.key(),
            accounts: cpi_accounts,
            data,
        };

        let mut account_infos = vec![
            self.vault_strategy_auth.clone(),
            self.strategy.clone(),
            self.vault_asset_mint.to_account_info(),
            self.vault_strategy_asset_ata.to_account_info(),
            self.asset_token_program.to_account_info(),
        ];

        account_infos.extend(remaining_accounts.iter().map(|acc| acc.to_account_info()));

        solana_invoke::invoke_signed(
            &instruction,
            &account_infos,
            &[&[
                VAULT_STRATEGY_AUTH_SEED,
                self.vault.key().as_ref(),
                self.strategy.key().as_ref(),
                &[vault_strategy_auth_bump],
            ]],
        )?;

        get_return_u64(instruction.program_id)
    }

    fn get_post_invoke_strategy_ata_amount(&mut self) -> Result<u64> {
        self.vault_strategy_asset_ata.reload()?;
        let post_invoke_strategy_ata_amount = self.vault_strategy_asset_ata.amount;
        Ok(post_invoke_strategy_ata_amount)
    }
}
