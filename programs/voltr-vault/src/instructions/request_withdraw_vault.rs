use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount as TokenkegAccount},
    token_2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    error::VaultError,
    state::{DisabledOperation, RequestWithdrawVaultEvent, Vault},
    utils::{
        constants::{PROTOCOL_SEED, REQUEST_WITHDRAW_VAULT_RECEIPT_SEED, VAULT_LP_MINT_SEED},
        get_amount_lp_to_burn_and_asset_to_withdraw,
    },
    OperationalState, Protocol, RequestWithdrawVaultReceipt,
};

pub fn handler_request_withdraw_vault(
    ctx: Context<RequestWithdrawVault>,
    amount: u64,
    is_amount_in_lp: bool,
    is_withdraw_all: bool,
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

    require!(amount > 0, VaultError::InvalidAmount);

    // Validate inputs
    let total_lp_supply = ctx.accounts.vault_lp_mint.supply;
    require!(total_lp_supply > 0, VaultError::InvalidAmount);

    let (
        vault_asset_mint,
        total_lp_supply_incl_fees,
        total_unlocked_asset,
        total_asset_value,
        redemption_fee,
        withdrawal_waiting_period,
    ) = {
        let vault = ctx.accounts.vault.load()?;
        (
            vault.asset.mint,
            vault.get_total_lp_supply_incl_fees(total_lp_supply)?,
            vault.get_unlocked_asset_value(u64::try_from(Clock::get()?.unix_timestamp)?)?,
            vault.get_total_asset_value(),
            vault.fee_configuration.redemption_fee,
            vault.vault_configuration.withdrawal_waiting_period,
        )
    };
    require!(total_unlocked_asset > 0, VaultError::InvalidAmount);

    let (amount_lp_to_burn, amount_asset_to_withdraw_decimal_bits) =
        get_amount_lp_to_burn_and_asset_to_withdraw(
            amount,
            ctx.accounts.user_lp_ata.amount,
            total_lp_supply_incl_fees,
            total_unlocked_asset,
            redemption_fee,
            is_amount_in_lp,
            is_withdraw_all,
        )?;
    // Transfer LP tokens to the receipt
    ctx.accounts
        .transfer_lp_user_to_receipt(amount_lp_to_burn)?;

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    // create the receipt
    let receipt = &mut ctx.accounts.request_withdraw_vault_receipt.load_init()?;
    receipt.init(
        ctx.accounts.vault.key(),
        ctx.accounts.user_transfer_authority.key(),
        amount_lp_to_burn,
        amount_asset_to_withdraw_decimal_bits,
        withdrawal_waiting_period,
        ctx.bumps.request_withdraw_vault_receipt,
        current_ts,
    )?;

    emit!(RequestWithdrawVaultEvent {
        vault: ctx.accounts.vault.key(),
        user: ctx.accounts.user_transfer_authority.key(),
        requested_amount: amount,
        is_amount_in_lp,
        is_withdraw_all,
        request_withdraw_vault_receipt: ctx.accounts.request_withdraw_vault_receipt.key(),
        amount_lp_escrowed: amount_lp_to_burn,
        amount_asset_to_withdraw_decimal_bits,
        withdrawable_from_ts: receipt.withdrawable_from_ts,
        vault_asset_mint,
        vault_asset_total_value_unlocked: total_unlocked_asset,
        vault_asset_total_value: total_asset_value,
        vault_lp_supply_incl_fees: total_lp_supply_incl_fees,
        requested_ts: current_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct RequestWithdrawVault<'info> {
    /// The payer of the request
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The authority that owns the LP tokens and wants to redeem them
    #[account()]
    pub user_transfer_authority: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account()]
    pub vault: AccountLoader<'info, Vault>,

    #[account(
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
        init,
        payer = payer,
        space = std::mem::size_of::<RequestWithdrawVaultReceipt>() + 8,
        seeds = [
            REQUEST_WITHDRAW_VAULT_RECEIPT_SEED,
            vault.key().as_ref(),
            user_transfer_authority.key().as_ref(),
        ],
        bump
    )]
    pub request_withdraw_vault_receipt: AccountLoader<'info, RequestWithdrawVaultReceipt>,
    pub lp_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> RequestWithdrawVault<'info> {
    fn transfer_lp_user_to_receipt(&self, amount: u64) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new(
                self.lp_token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.user_lp_ata.to_account_info(),
                    to: self.request_withdraw_lp_ata.to_account_info(),
                    authority: self.user_transfer_authority.to_account_info(),
                    mint: self.vault_lp_mint.to_account_info(),
                },
            ),
            amount,
            self.vault_lp_mint.decimals,
        )
    }
}
