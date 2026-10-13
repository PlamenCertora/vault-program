use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount as TokenkegAccount},
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    error::VaultError,
    state::{DepositVaultEvent, DisabledOperation, Vault},
    utils::{
        calc_deposit_lp_to_mint, calc_init_lp_to_mint,
        constants::{
            PROTOCOL_SEED, VAULT_ASSET_IDLE_AUTH_SEED, VAULT_LP_MINT_AUTH_SEED, VAULT_LP_MINT_SEED,
        },
        SafeMath,
    },
    OperationalState, Protocol,
};

pub fn handler_deposit_vault(ctx: Context<DepositVault>, amount: u64) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::DEPOSIT_VAULT)?;
    ctx.accounts
        .vault
        .load()?
        .vault_configuration
        .check_operation_disabled(DisabledOperation::DEPOSIT_VAULT)?;

    require!(amount > 0, VaultError::InvalidAmount);

    // Check if max cap is not exceeded
    let (
        max_cap,
        total_asset_value_before,
        vault_highest_asset_per_lp_decimal_bits_before,
        active_from_ts,
        total_lp_supply_incl_fees_before,
        issuance_fee,
    ) = {
        let vault = &ctx.accounts.vault.load()?;
        (
            vault.vault_configuration.max_cap,
            vault.get_total_asset_value(),
            vault
                .high_water_mark
                .get_highest_asset_per_lp_decimal_bits()?,
            vault.vault_configuration.start_at_ts,
            vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?,
            vault.fee_configuration.issuance_fee,
        )
    };

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    require!(
        max_cap >= total_asset_value_before.safe_add(amount)?,
        VaultError::MaxCapExceeded
    );
    require!(current_ts >= active_from_ts, VaultError::VaultNotActive);

    // 1. Transfer asset tokens from user to vault idle
    ctx.accounts.transfer_asset_user_to_idle(amount)?;

    let vault = &mut ctx.accounts.vault.load_mut()?;
    vault.handle_management_fee(
        current_ts,
        total_asset_value_before,
        total_lp_supply_incl_fees_before,
    )?;

    let total_lp_supply_incl_fees_after_management_fee =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    let lp_to_mint_before_dead_weight = if total_lp_supply_incl_fees_before == 0 {
        calc_init_lp_to_mint(
            amount,
            ctx.accounts.vault_asset_mint.decimals,
            ctx.accounts.vault_lp_mint.decimals,
        )?
    } else {
        calc_deposit_lp_to_mint(
            amount,
            total_lp_supply_incl_fees_after_management_fee,
            total_asset_value_before,
            issuance_fee,
        )?
    };

    let lp_to_mint =
        vault.handle_dead_weight_attribution_idempotent(lp_to_mint_before_dead_weight)?;

    // 2. Mint LP tokens to the user's LP ATA
    ctx.accounts
        .mint_lp_to_user(lp_to_mint, vault.lp.mint_auth_bump)?;

    // Update the vault
    ctx.accounts.vault_lp_mint.reload()?;
    vault.increment_total_value(amount)?;
    vault.update_high_water_mark_idempotent(ctx.accounts.vault_lp_mint.supply, current_ts)?;
    vault.update_last_updated_ts(current_ts)?;

    emit!(DepositVaultEvent {
        user: ctx.accounts.user_transfer_authority.key(),
        user_amount_asset_deposited: amount,
        user_amount_lp_minted: lp_to_mint,
        vault: ctx.accounts.vault.key(),
        vault_asset_mint: ctx.accounts.vault_asset_mint.key(),
        vault_asset_total_value_before: total_asset_value_before,
        vault_asset_total_value_after: vault.get_total_asset_value(),
        vault_lp_supply_incl_fees_before: total_lp_supply_incl_fees_before,
        vault_lp_supply_incl_fees_after: vault
            .get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after: vault
            .high_water_mark
            .get_highest_asset_per_lp_decimal_bits()?,
        deposited_ts: vault.last_updated_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct DepositVault<'info> {
    #[account()]
    pub user_transfer_authority: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(
        mut,
        constraint = vault.load()?.asset.mint == vault_asset_mint.key() @ VaultError::InvalidTokenMint,
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

    /// The user's asset ATA from which they are depositing tokens.
    #[account(
        mut,
        associated_token::mint = vault_asset_mint,
        associated_token::authority = user_transfer_authority,
        associated_token::token_program = asset_token_program,
    )]
    pub user_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,

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
        seeds = [VAULT_ASSET_IDLE_AUTH_SEED, vault.key().as_ref()],
        bump = vault.load()?.asset.idle_ata_auth_bump
    )]
    pub vault_asset_idle_auth: AccountInfo<'info>,

    /// The user's LP ATA where we will mint LP tokens.
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = user_transfer_authority,
        associated_token::token_program = lp_token_program,
    )]
    pub user_lp_ata: Account<'info, TokenkegAccount>,

    /// The PDA authority used to sign mint instructions for LP tokens.
    /// CHECK:
    #[account(
        seeds = [VAULT_LP_MINT_AUTH_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_auth_bump
    )]
    pub vault_lp_mint_auth: AccountInfo<'info>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub lp_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> DepositVault<'info> {
    fn transfer_asset_user_to_idle(&self, amount: u64) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new(
                self.asset_token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.user_asset_ata.to_account_info(),
                    to: self.vault_asset_idle_ata.to_account_info(),
                    authority: self.user_transfer_authority.to_account_info(),
                    mint: self.vault_asset_mint.to_account_info(),
                },
            ),
            amount,
            self.vault_asset_mint.decimals,
        )
    }

    fn mint_lp_to_user(&self, amount: u64, vault_lp_mint_auth_bump: u8) -> Result<()> {
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                self.lp_token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: self.vault_lp_mint.to_account_info(),
                    to: self.user_lp_ata.to_account_info(),
                    authority: self.vault_lp_mint_auth.to_account_info(),
                },
                &[&[
                    VAULT_LP_MINT_AUTH_SEED,
                    &self.vault.key().to_bytes(),
                    &[vault_lp_mint_auth_bump],
                ]],
            ),
            amount,
        )
    }
}
