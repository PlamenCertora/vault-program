use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount as TokenkegAccount},
    token_2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    state::{CancelRequestWithdrawVaultEvent, DisabledOperation, Vault},
    utils::{
        calc_cancel_withdraw_amount_lp_to_refund_and_burn,
        constants::{PROTOCOL_SEED, REQUEST_WITHDRAW_VAULT_RECEIPT_SEED, VAULT_LP_MINT_SEED},
    },
    OperationalState, Protocol, RequestWithdrawVaultReceipt,
};

pub fn handler_cancel_request_withdraw_vault(
    ctx: Context<CancelRequestWithdrawVault>,
) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::WITHDRAW_VAULT)?;
    ctx.accounts
        .vault
        .load()?
        .vault_configuration
        .check_operation_disabled(DisabledOperation::WITHDRAW_VAULT)?;

    let mut vault = ctx.accounts.vault.load_mut()?;
    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;
    let total_unlocked_asset = vault.get_unlocked_asset_value(current_ts)?;

    // Get the current total LP supply with fees
    let total_lp_supply_incl_fees =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    let (
        initial_amount_lp_to_refund,
        initial_amount_asset_worth_decimal_bits,
        request_withdraw_vault_receipt_bump,
    ) = {
        let receipt = ctx.accounts.request_withdraw_vault_receipt.load()?;
        (
            receipt.amount_lp_escrowed,
            receipt.amount_asset_to_withdraw_decimal_bits,
            receipt.bump,
        )
    };

    let (amount_lp_to_refund, amount_lp_to_burn) =
        calc_cancel_withdraw_amount_lp_to_refund_and_burn(
            initial_amount_lp_to_refund,
            initial_amount_asset_worth_decimal_bits,
            total_lp_supply_incl_fees,
            total_unlocked_asset,
            vault.fee_configuration.redemption_fee,
        )?;

    // Refund LP tokens to the user
    ctx.accounts
        .transfer_lp_receipt_to_user(amount_lp_to_refund, request_withdraw_vault_receipt_bump)?;

    if amount_lp_to_burn > 0 {
        // Burn the LP tokens
        ctx.accounts
            .burn_lp_from_receipt(amount_lp_to_burn, request_withdraw_vault_receipt_bump)?;
    }

    let vault_highest_asset_per_lp_decimal_bits_before = vault
        .high_water_mark
        .get_highest_asset_per_lp_decimal_bits()?;
    ctx.accounts.vault_lp_mint.reload()?;
    vault.update_high_water_mark_idempotent(ctx.accounts.vault_lp_mint.supply, current_ts)?;
    let vault_highest_asset_per_lp_decimal_bits_after = vault
        .high_water_mark
        .get_highest_asset_per_lp_decimal_bits()?;

    emit!(CancelRequestWithdrawVaultEvent {
        vault: ctx.accounts.vault.key(),
        user: ctx.accounts.user_transfer_authority.key(),
        request_withdraw_vault_receipt: ctx.accounts.request_withdraw_vault_receipt.key(),
        amount_lp_refunded: amount_lp_to_refund,
        amount_lp_burned: amount_lp_to_burn,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after,
        cancelled_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct CancelRequestWithdrawVault<'info> {
    /// The authority that owns the LP tokens and wants to redeem them
    #[account(mut)]
    pub user_transfer_authority: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(mut)]
    pub vault: AccountLoader<'info, Vault>,

    #[account(
        mut,
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,

    /// The user's LP token account from which LP tokens will be burned.
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = user_transfer_authority,
        associated_token::token_program = lp_token_program,
    )]
    pub user_lp_ata: Account<'info, TokenkegAccount>,

    /// The request's associated token account for LP.
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = request_withdraw_vault_receipt,
        associated_token::token_program = lp_token_program,
    )]
    pub request_withdraw_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

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
    pub lp_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> CancelRequestWithdrawVault<'info> {
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

    fn transfer_lp_receipt_to_user(
        &self,
        amount: u64,
        request_withdraw_vault_receipt_bump: u8,
    ) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.lp_token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.request_withdraw_lp_ata.to_account_info(),
                    to: self.user_lp_ata.to_account_info(),
                    authority: self.request_withdraw_vault_receipt.to_account_info(),
                    mint: self.vault_lp_mint.to_account_info(),
                },
                &[&[
                    REQUEST_WITHDRAW_VAULT_RECEIPT_SEED,
                    self.vault.key().as_ref(),
                    self.user_transfer_authority.key().as_ref(),
                    &[request_withdraw_vault_receipt_bump],
                ]],
            ),
            amount,
            self.vault_lp_mint.decimals,
        )
    }
}
