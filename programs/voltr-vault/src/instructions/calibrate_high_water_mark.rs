use anchor_lang::prelude::*;
use anchor_spl::{
    token::Token,
    token_2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    state::CalibrateHighWaterMarkEvent,
    utils::constants::{PROTOCOL_SEED, VAULT_LP_MINT_SEED},
    OperationalState, Protocol, Vault,
};

pub fn handler_calibrate_high_water_mark(ctx: Context<CalibrateHighWaterMark>) -> Result<()> {
    // AKIN TO UPDATING VAULT
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::UPDATE_VAULT)?;
    let vault = &mut ctx.accounts.vault.load_mut()?;
    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    let (
        vault_asset_total_value,
        vault_lp_supply_incl_fees_before,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after,
        lp_to_burn,
    ) = vault.calibrate_high_water_mark(ctx.accounts.vault_lp_mint.supply, current_ts)?;

    ctx.accounts.burn_lp_from_admin(lp_to_burn)?;

    ctx.accounts.vault_lp_mint.reload()?;
    let vault_lp_supply_incl_fees_after =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    emit!(CalibrateHighWaterMarkEvent {
        admin: ctx.accounts.admin.key(),
        vault: ctx.accounts.vault.key(),
        vault_asset_total_value,
        vault_lp_supply_incl_fees_before,
        vault_lp_supply_incl_fees_after,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after,
        amount_lp_burned: lp_to_burn,
        calibrated_ts: current_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct CalibrateHighWaterMark<'info> {
    #[account()]
    pub admin: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(
        mut,
        has_one = admin
    )]
    pub vault: AccountLoader<'info, Vault>,

    #[account(
        mut,
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = admin,
        associated_token::token_program = lp_token_program,
    )]
    pub admin_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub lp_token_program: Program<'info, Token>,
}

impl<'info> CalibrateHighWaterMark<'info> {
    fn burn_lp_from_admin(&self, amount: u64) -> Result<()> {
        token_2022::burn(
            CpiContext::new(
                self.lp_token_program.to_account_info(),
                token_2022::Burn {
                    mint: self.vault_lp_mint.to_account_info(),
                    from: self.admin_lp_ata.to_account_info(),
                    authority: self.admin.to_account_info(),
                },
            ),
            amount,
        )
    }
}
