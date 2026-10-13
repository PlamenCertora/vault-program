use anchor_lang::prelude::*;
use anchor_spl::{
    token::Token,
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    error::VaultError,
    state::{DisabledOperation, Vault, WithdrawVaultEvent},
    utils::{
        constants::{
            PROTOCOL_SEED, REQUEST_WITHDRAW_VAULT_RECEIPT_SEED, VAULT_ASSET_IDLE_AUTH_SEED,
            VAULT_LP_MINT_SEED,
        },
        get_capped_withdraw_asset_to_redeem,
    },
    OperationalState, Protocol, RequestWithdrawVaultReceipt,
};

pub fn handler_withdraw_vault(ctx: Context<WithdrawVault>) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::WITHDRAW_VAULT)?;
    ctx.accounts
        .vault
        .load()?
        .vault_configuration
        .check_operation_disabled(DisabledOperation::WITHDRAW_VAULT)?;

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    // Validate inputs
    let receipt = ctx.accounts.request_withdraw_vault_receipt.load()?;
    require!(
        receipt.is_withdrawable(current_ts)?,
        VaultError::WithdrawalNotYetAvailable
    );

    let total_lp_supply = ctx.accounts.vault_lp_mint.supply;
    require!(total_lp_supply > 0, VaultError::InvalidAmount);

    let (
        total_lp_supply_incl_fees_before,
        total_asset_value_before,
        total_unlocked_asset,
        vault_highest_asset_per_lp_decimal_bits_before,
        redemption_fee,
        idle_ata_auth_bump,
    ) = {
        let vault = &ctx.accounts.vault.load()?;
        (
            vault.get_total_lp_supply_incl_fees(total_lp_supply)?,
            vault.get_total_asset_value(),
            vault.get_unlocked_asset_value(current_ts)?,
            vault
                .high_water_mark
                .get_highest_asset_per_lp_decimal_bits()?,
            vault.fee_configuration.redemption_fee,
            vault.asset.idle_ata_auth_bump,
        )
    };
    require!(total_unlocked_asset > 0, VaultError::InvalidAmount);

    let amount_lp_to_burn = receipt.amount_lp_escrowed;
    let initial_amount_asset_to_withdraw_decimal_bits =
        receipt.amount_asset_to_withdraw_decimal_bits;

    let vault = &mut ctx.accounts.vault.load_mut()?;
    vault.handle_management_fee(
        current_ts,
        total_asset_value_before,
        total_lp_supply_incl_fees_before,
    )?;

    let total_lp_supply_incl_fees_after_management_fee =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    let amount_asset_to_withdraw = get_capped_withdraw_asset_to_redeem(
        initial_amount_asset_to_withdraw_decimal_bits,
        amount_lp_to_burn,
        total_lp_supply_incl_fees_after_management_fee,
        total_unlocked_asset,
        redemption_fee,
    )?;
    // 1. Burn the user's LP tokens
    ctx.accounts
        .burn_lp_from_receipt(amount_lp_to_burn, receipt.bump)?;

    // 2. Transfer redeemable asset tokens to the user
    ctx.accounts
        .transfer_asset_idle_to_user(amount_asset_to_withdraw, idle_ata_auth_bump)?;

    // Update the vault
    ctx.accounts.vault_lp_mint.reload()?;
    vault.decrement_total_value(amount_asset_to_withdraw)?;
    vault.update_high_water_mark_idempotent(ctx.accounts.vault_lp_mint.supply, current_ts)?;
    vault.update_last_updated_ts(current_ts)?;

    emit!(WithdrawVaultEvent {
        user: ctx.accounts.user_transfer_authority.key(),
        user_amount_asset_withdrawn: amount_asset_to_withdraw,
        user_amount_lp_burned: amount_lp_to_burn,
        vault: ctx.accounts.vault.key(),
        vault_asset_mint: ctx.accounts.vault_asset_mint.key(),
        vault_asset_total_value_unlocked_before: total_unlocked_asset,
        vault_asset_total_value_before: total_asset_value_before,
        vault_asset_total_value_after: vault.get_total_asset_value(),
        vault_lp_supply_incl_fees_before: total_lp_supply_incl_fees_before,
        vault_lp_supply_incl_fees_after: vault
            .get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after: vault
            .high_water_mark
            .get_highest_asset_per_lp_decimal_bits()?,
        withdrawn_ts: vault.last_updated_ts,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawVault<'info> {
    /// The authority that owns the LP tokens and wants to redeem them
    #[account(mut)]
    pub user_transfer_authority: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(
        mut,
        constraint = vault.load()?.asset.mint == vault_asset_mint.key()
    )]
    pub vault: AccountLoader<'info, Vault>,

    #[account(mint::token_program = asset_token_program)]
    pub vault_asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,

    /// The request's LP token account from which LP tokens will be burned.
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = request_withdraw_vault_receipt,
        associated_token::token_program = lp_token_program,
    )]
    pub request_withdraw_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

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
        seeds = [VAULT_ASSET_IDLE_AUTH_SEED, vault.key().as_ref()],
        bump = vault.load()?.asset.idle_ata_auth_bump
    )]
    pub vault_asset_idle_auth: AccountInfo<'info>,

    /// The user's asset ATA to which asset tokens will be sent.
    #[account(
        mut,
        associated_token::mint = vault_asset_mint,
        associated_token::authority = user_transfer_authority,
        associated_token::token_program = asset_token_program,
    )]
    pub user_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(
        mut,
        seeds = [
            REQUEST_WITHDRAW_VAULT_RECEIPT_SEED,
            vault.key().as_ref(),
            user_transfer_authority.key().as_ref(),
        ],
        bump = request_withdraw_vault_receipt.load()?.bump,
        close = user_transfer_authority
    )]
    pub request_withdraw_vault_receipt: AccountLoader<'info, RequestWithdrawVaultReceipt>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub lp_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawVault<'info> {
    fn burn_lp_from_receipt(
        &self,
        amount: u64,
        request_withdraw_vault_receipt_bump: u8,
    ) -> Result<()> {
        token_2022::burn(
            CpiContext::new_with_signer(
                self.lp_token_program.to_account_info(),
                token_2022::Burn {
                    mint: self.vault_lp_mint.to_account_info(),
                    from: self.request_withdraw_lp_ata.to_account_info(),
                    authority: self.request_withdraw_vault_receipt.to_account_info(),
                },
                &[&[
                    REQUEST_WITHDRAW_VAULT_RECEIPT_SEED,
                    self.vault.key().as_ref(),
                    self.user_transfer_authority.key().as_ref(),
                    &[request_withdraw_vault_receipt_bump],
                ]],
            ),
            amount,
        )
    }

    fn transfer_asset_idle_to_user(&self, amount: u64, idle_ata_auth_bump: u8) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.vault_asset_idle_ata.to_account_info(),
                    to: self.user_asset_ata.to_account_info(),
                    authority: self.vault_asset_idle_auth.to_account_info(),
                    mint: self.vault_asset_mint.to_account_info(),
                },
                &[&[
                    VAULT_ASSET_IDLE_AUTH_SEED,
                    self.vault.key().as_ref(),
                    &[idle_ata_auth_bump],
                ]],
            ),
            amount,
            self.vault_asset_mint.decimals,
        )
    }
}
