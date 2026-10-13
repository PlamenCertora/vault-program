use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use borsh::BorshDeserialize;

use crate::{
    error::VaultError,
    state::UpdateVaultConfigEvent,
    utils::{
        constants::{MAX_FEE_BPS, PROTOCOL_SEED},
        SafeMath,
    },
    OperationalState, Protocol, Vault,
};

#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum VaultConfigField {
    MaxCap,
    StartAtTs,
    LockedProfitDegradationDuration,
    WithdrawalWaitingPeriod,
    ManagerPerformanceFee,
    AdminPerformanceFee,
    ManagerManagementFee,
    AdminManagementFee,
    RedemptionFee,
    IssuanceFee,
    Manager,
    DisabledOperations,
}

pub fn handler_update_vault_config(
    ctx: Context<UpdateVaultConfig>,
    field: VaultConfigField,
    data: Vec<u8>,
) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::UPDATE_VAULT)?;

    let vault = &mut ctx.accounts.vault.load_mut()?;
    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    let requires_fee_charge = matches!(
        field,
        VaultConfigField::ManagerManagementFee | VaultConfigField::AdminManagementFee
    );

    if requires_fee_charge {
        let lp_mint_account_info = &ctx.remaining_accounts[0];
        let lp_mint = Mint::try_deserialize(&mut &lp_mint_account_info.data.borrow()[..])?;

        require!(
            lp_mint_account_info.key() == vault.lp.mint,
            VaultError::InvalidTokenMint
        );

        let total_asset_value = vault.get_total_asset_value();
        let total_lp_supply_incl_fees = vault.get_total_lp_supply_incl_fees(lp_mint.supply)?;

        vault.handle_management_fee(current_ts, total_asset_value, total_lp_supply_incl_fees)?;
    }

    let (field_name, old_value, new_value) = match field {
        VaultConfigField::MaxCap => {
            let new_val: u64 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.vault_configuration.max_cap;

            vault.vault_configuration.max_cap = new_val;
            ("MaxCap", old_val.to_string(), new_val.to_string())
        }

        VaultConfigField::StartAtTs => {
            let new_val: u64 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.vault_configuration.start_at_ts;

            vault.vault_configuration.start_at_ts = new_val;
            ("StartAtTs", old_val.to_string(), new_val.to_string())
        }

        VaultConfigField::LockedProfitDegradationDuration => {
            let new_val: u64 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.vault_configuration.locked_profit_degradation_duration;

            vault.vault_configuration.locked_profit_degradation_duration = new_val;
            (
                "LockedProfitDegradationDuration",
                old_val.to_string(),
                new_val.to_string(),
            )
        }

        VaultConfigField::WithdrawalWaitingPeriod => {
            let new_val: u64 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.vault_configuration.withdrawal_waiting_period;

            vault.vault_configuration.withdrawal_waiting_period = new_val;
            (
                "WithdrawalWaitingPeriod",
                old_val.to_string(),
                new_val.to_string(),
            )
        }

        VaultConfigField::ManagerPerformanceFee => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.fee_configuration.manager_performance_fee;

            // Validate total performance fee
            let total_perf_fee = new_val
                .safe_add(vault.fee_configuration.admin_performance_fee)?
                .safe_add(vault.fee_configuration.protocol_performance_fee)?;
            require!(
                total_perf_fee <= MAX_FEE_BPS,
                VaultError::InvalidFeeConfiguration
            );

            vault.fee_configuration.manager_performance_fee = new_val;
            (
                "ManagerPerformanceFee",
                old_val.to_string(),
                new_val.to_string(),
            )
        }

        VaultConfigField::AdminPerformanceFee => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.fee_configuration.admin_performance_fee;

            // Validate total performance fee
            let total_perf_fee = new_val
                .safe_add(vault.fee_configuration.manager_performance_fee)?
                .safe_add(vault.fee_configuration.protocol_performance_fee)?;
            require!(
                total_perf_fee <= MAX_FEE_BPS,
                VaultError::InvalidFeeConfiguration
            );

            vault.fee_configuration.admin_performance_fee = new_val;
            (
                "AdminPerformanceFee",
                old_val.to_string(),
                new_val.to_string(),
            )
        }

        VaultConfigField::ManagerManagementFee => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.fee_configuration.manager_management_fee;

            // Validate total management fee
            let total_mgmt_fee = new_val
                .safe_add(vault.fee_configuration.admin_management_fee)?
                .safe_add(vault.fee_configuration.protocol_management_fee)?;
            require!(
                total_mgmt_fee <= MAX_FEE_BPS,
                VaultError::InvalidFeeConfiguration
            );

            vault.fee_configuration.manager_management_fee = new_val;
            vault.fee_update.last_management_fee_update_ts = current_ts;

            (
                "ManagerManagementFee",
                old_val.to_string(),
                new_val.to_string(),
            )
        }

        VaultConfigField::AdminManagementFee => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.fee_configuration.admin_management_fee;

            // Validate total management fee
            let total_mgmt_fee = new_val
                .safe_add(vault.fee_configuration.manager_management_fee)?
                .safe_add(vault.fee_configuration.protocol_management_fee)?;
            require!(
                total_mgmt_fee <= MAX_FEE_BPS,
                VaultError::InvalidFeeConfiguration
            );

            vault.fee_configuration.admin_management_fee = new_val;
            vault.fee_update.last_management_fee_update_ts = current_ts;

            (
                "AdminManagementFee",
                old_val.to_string(),
                new_val.to_string(),
            )
        }

        VaultConfigField::RedemptionFee => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.fee_configuration.redemption_fee;

            require!(new_val <= MAX_FEE_BPS, VaultError::InvalidFeeConfiguration);

            vault.fee_configuration.redemption_fee = new_val;
            ("RedemptionFee", old_val.to_string(), new_val.to_string())
        }

        VaultConfigField::IssuanceFee => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.fee_configuration.issuance_fee;

            require!(new_val <= MAX_FEE_BPS, VaultError::InvalidFeeConfiguration);

            vault.fee_configuration.issuance_fee = new_val;
            ("IssuanceFee", old_val.to_string(), new_val.to_string())
        }

        VaultConfigField::Manager => {
            let new_val: Pubkey = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.manager;

            vault.manager = new_val;
            ("Manager", old_val.to_string(), new_val.to_string())
        }

        VaultConfigField::DisabledOperations => {
            let new_val: u16 = BorshDeserialize::try_from_slice(&data)?;
            let old_val = vault.vault_configuration.disabled_operations;

            vault.vault_configuration.disabled_operations = new_val;
            (
                "DisabledOperations",
                old_val.to_string(),
                new_val.to_string(),
            )
        }
    };

    vault.version = 4;
    vault.last_updated_ts = current_ts;

    emit!(UpdateVaultConfigEvent {
        admin: ctx.accounts.admin.key(),
        vault: ctx.accounts.vault.key(),
        field: field_name.to_string(),
        old_value,
        new_value,
        updated_ts: current_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateVaultConfig<'info> {
    /// CHECK:
    #[account()]
    pub admin: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(mut, has_one = admin)]
    pub vault: AccountLoader<'info, Vault>,

    pub rent: Sysvar<'info, Rent>,
    // remaining_accounts:
    // [0] vault_lp_mint: InterfaceAccount<'info, Mint> (REQUIRED when updating ManagerManagementFee or AdminManagementFee)
}
