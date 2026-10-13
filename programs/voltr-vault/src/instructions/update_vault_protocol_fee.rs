use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{
    error::VaultError,
    state::{UpdateVaultProtocolFeeEvent, Vault},
    utils::{constants::{MAX_FEE_BPS, PROTOCOL_SEED, VAULT_LP_MINT_SEED}, SafeMath},
    OperationalState, Protocol,
};

#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum ProtocolFeeField {
    ProtocolPerformanceFee,
    ProtocolManagementFee,
}

pub fn handler_update_vault_protocol_fee(
    ctx: Context<UpdateVaultProtocolFee>,
    field: ProtocolFeeField,
    fee_bps: u16,
) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::UPDATE_VAULT)?;

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    // Initialize the vault account
    let vault = &mut ctx.accounts.vault.load_mut()?;
    let (field_name, old_value, new_value) = match field {
        ProtocolFeeField::ProtocolPerformanceFee => {
            let admin_performance_fee_bps = vault.fee_configuration.admin_performance_fee;
            let manager_performance_fee_bps = vault.fee_configuration.manager_performance_fee;
            let old_value = vault.fee_configuration.protocol_performance_fee;
            let new_value = fee_bps;
            let total_performance_fee = new_value.safe_add(admin_performance_fee_bps)?.safe_add(manager_performance_fee_bps)?;
            require!(total_performance_fee <= MAX_FEE_BPS, VaultError::InvalidFeeConfiguration);

            vault.fee_configuration.protocol_performance_fee = new_value;
            ("ProtocolPerformanceFee", old_value, new_value)
        }
        ProtocolFeeField::ProtocolManagementFee => {
            let total_asset_value = vault.get_total_asset_value();
            let total_lp_supply_incl_fees = vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;
    
            vault.handle_management_fee(current_ts, total_asset_value, total_lp_supply_incl_fees)?;

            let admin_management_fee_bps = vault.fee_configuration.admin_management_fee;
            let manager_management_fee_bps = vault.fee_configuration.manager_management_fee;
            let old_value = vault.fee_configuration.protocol_management_fee;
            let new_value = fee_bps;
            let total_management_fee = new_value.safe_add(admin_management_fee_bps)?.safe_add(manager_management_fee_bps)?;
            require!(total_management_fee <= MAX_FEE_BPS, VaultError::InvalidFeeConfiguration);

            vault.fee_configuration.protocol_management_fee = new_value;
            ("ProtocolManagementFee", old_value, new_value)
        }
    };
    
    vault.version = 4;
    vault.last_updated_ts = current_ts;

    emit!(UpdateVaultProtocolFeeEvent {
        admin: ctx.accounts.admin.key(),
        vault: ctx.accounts.vault.key(),
        field: field_name.to_string(),
        old_value,
        new_value,
        updated_ts: vault.last_updated_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateVaultProtocolFee<'info> {
    /// CHECK:
    #[account()]
    pub admin: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump, has_one = admin)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(mut)]
    pub vault: AccountLoader<'info, Vault>,

    #[account(mut, 
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,
}
