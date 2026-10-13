use anchor_lang::prelude::*;
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    state::{DisabledOperation, HarvestFeeEvent, Vault},
    utils::constants::{PROTOCOL_SEED, VAULT_LP_MINT_AUTH_SEED, VAULT_LP_MINT_SEED},
    OperationalState, Protocol,
};

#[derive(Accounts)]
pub struct HarvestFee<'info> {
    /// CHECK: anyone can harvest
    #[account()]
    pub harvester: Signer<'info>,
    /// CHECK:
    #[account(constraint = vault_manager.key() == vault.load()?.manager)]
    pub vault_manager: AccountInfo<'info>,
    /// CHECK:
    #[account(constraint = vault_admin.key() == vault.load()?.admin)]
    pub vault_admin: AccountInfo<'info>,
    /// CHECK:
    #[account(constraint = protocol_admin.key() == protocol.load()?.admin)]
    pub protocol_admin: AccountInfo<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(mut)]
    pub vault: AccountLoader<'info, Vault>,

    /// CHECK:
    #[account(
        mut,
        mint::token_program = lp_token_program,
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,

    /// CHECK:
    #[account(
        seeds = [VAULT_LP_MINT_AUTH_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_auth_bump
    )]
    pub vault_lp_mint_auth: AccountInfo<'info>,

    /// CHECK:
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = vault_manager,
        associated_token::token_program = lp_token_program,
    )]
    pub vault_manager_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = vault_admin,
        associated_token::token_program = lp_token_program,
    )]
    pub vault_admin_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = protocol_admin,
        associated_token::token_program = lp_token_program,
    )]
    pub protocol_admin_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub lp_token_program: Program<'info, Token>,
}

pub fn handler_harvest_fee(ctx: Context<HarvestFee>) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::HARVEST_FEE)?;
    ctx.accounts
        .vault
        .load()?
        .vault_configuration
        .check_operation_disabled(DisabledOperation::HARVEST_FEE)?;

    let vault = &mut ctx.accounts.vault.load_mut()?;
    let total_asset_value = vault.get_total_asset_value();
    let total_lp_supply_incl_fees =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    vault.handle_management_fee(
        u64::try_from(Clock::get()?.unix_timestamp)?,
        total_asset_value,
        total_lp_supply_incl_fees,
    )?;

    let (amount_lp_admin_fees, amount_lp_manager_fees, amount_lp_protocol_fees) = {
        (
            vault.fee_state.accumulated_lp_admin_fees,
            vault.fee_state.accumulated_lp_manager_fees,
            vault.fee_state.accumulated_lp_protocol_fees,
        )
    };

    ctx.accounts.mint_fee_to_accounts(
        amount_lp_admin_fees,
        amount_lp_manager_fees,
        amount_lp_protocol_fees,
        vault.lp.mint_auth_bump,
    )?;
    vault.reset_fee_state()?;

    emit!(HarvestFeeEvent {
        harvester: ctx.accounts.harvester.key(),
        vault: ctx.accounts.vault.key(),
        protocol: ctx.accounts.protocol.key(),
        admin: ctx.accounts.vault_admin.key(),
        manager: ctx.accounts.vault_manager.key(),
        amount_lp_admin_fees,
        amount_lp_manager_fees,
        amount_lp_protocol_fees,
    });

    Ok(())
}

impl<'info> HarvestFee<'info> {
    fn mint_lp_to_account(
        &self,
        to: AccountInfo<'info>,
        amount: u64,
        vault_lp_mint_auth_bump: u8,
    ) -> Result<()> {
        anchor_spl::token_interface::mint_to(
            CpiContext::new_with_signer(
                self.lp_token_program.to_account_info(),
                anchor_spl::token_interface::MintTo {
                    mint: self.vault_lp_mint.to_account_info(),
                    to,
                    authority: self.vault_lp_mint_auth.to_account_info(),
                },
                &[&[
                    VAULT_LP_MINT_AUTH_SEED,
                    self.vault.key().as_ref(),
                    &[vault_lp_mint_auth_bump],
                ]],
            ),
            amount,
        )
    }

    fn mint_fee_to_accounts(
        &self,
        amount_lp_admin_fees: u64,
        amount_lp_manager_fees: u64,
        amount_lp_protocol_fees: u64,
        vault_lp_mint_auth_bump: u8,
    ) -> Result<()> {
        self.mint_lp_to_account(
            self.vault_admin_lp_ata.to_account_info(),
            amount_lp_admin_fees,
            vault_lp_mint_auth_bump,
        )?;
        self.mint_lp_to_account(
            self.vault_manager_lp_ata.to_account_info(),
            amount_lp_manager_fees,
            vault_lp_mint_auth_bump,
        )?;
        self.mint_lp_to_account(
            self.protocol_admin_lp_ata.to_account_info(),
            amount_lp_protocol_fees,
            vault_lp_mint_auth_bump,
        )?;
        Ok(())
    }
}
