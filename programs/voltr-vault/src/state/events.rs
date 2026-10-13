use anchor_lang::prelude::*;

#[event]
#[derive(Default)]
pub struct InitializeVaultEvent {
    pub payer: Pubkey,
    pub vault: Pubkey,
    pub vault_name: String,
    pub vault_description: String,
    pub vault_asset_mint: Pubkey,
    pub vault_asset_idle_ata: Pubkey,
    pub vault_lp_mint: Pubkey,
    pub vault_manager: Pubkey,
    pub vault_admin: Pubkey,
    pub vault_config_max_cap: u64,
    pub vault_config_start_at_ts: u64,
    pub vault_config_locked_profit_degradation_duration: u64,
    pub vault_config_withdrawal_waiting_period: u64,
    pub vault_config_manager_performance_fee: u16,
    pub vault_config_admin_performance_fee: u16,
    pub vault_config_manager_management_fee: u16,
    pub vault_config_admin_management_fee: u16,
    pub vault_config_redemption_fee: u16,
    pub vault_config_issuance_fee: u16,
    pub initialized_ts: u64,
}

#[event]
#[derive(Default)]
pub struct UpdateVaultEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub vault_config_max_cap_before: u64,
    pub vault_config_max_cap_after: u64,
    pub vault_config_start_at_ts_before: u64,
    pub vault_config_start_at_ts_after: u64,
    pub vault_config_locked_profit_degradation_duration_before: u64,
    pub vault_config_locked_profit_degradation_duration_after: u64,
    pub vault_config_withdrawal_waiting_period_before: u64,
    pub vault_config_withdrawal_waiting_period_after: u64,
    pub vault_config_manager_performance_fee_before: u16,
    pub vault_config_manager_performance_fee_after: u16,
    pub vault_config_admin_performance_fee_before: u16,
    pub vault_config_admin_performance_fee_after: u16,
    pub vault_config_manager_management_fee_before: u16,
    pub vault_config_manager_management_fee_after: u16,
    pub vault_config_admin_management_fee_before: u16,
    pub vault_config_admin_management_fee_after: u16,
    pub vault_config_redemption_fee_before: u16,
    pub vault_config_redemption_fee_after: u16,
    pub vault_config_issuance_fee_before: u16,
    pub vault_config_issuance_fee_after: u16,
    pub updated_ts: u64,
}

#[event]
#[derive(Default)]
pub struct AddAdaptorEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub adaptor_program: Pubkey,
    pub adaptor_add_receipt: Pubkey,
    pub adaptor_last_updated_ts: u64,
    pub added_ts: u64,
}

#[event]
#[derive(Default)]
pub struct InitializeStrategyEvent {
    pub manager: Pubkey,
    pub vault: Pubkey,
    pub vault_strategy_auth: Pubkey,
    pub strategy: Pubkey,
    pub strategy_init_receipt: Pubkey,
    pub adaptor_program: Pubkey,
    pub initialized_ts: u64,
    pub is_initialized: bool,
}

#[event]
#[derive(Default)]
pub struct DepositVaultEvent {
    pub user: Pubkey,
    pub user_amount_asset_deposited: u64,
    pub user_amount_lp_minted: u64,
    pub vault: Pubkey,
    pub vault_asset_mint: Pubkey,
    pub vault_asset_total_value_before: u64,
    pub vault_asset_total_value_after: u64,
    pub vault_lp_supply_incl_fees_before: u64,
    pub vault_lp_supply_incl_fees_after: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub deposited_ts: u64,
}

#[event]
#[derive(Default)]
pub struct WithdrawVaultEvent {
    pub user: Pubkey,
    pub user_amount_asset_withdrawn: u64,
    pub user_amount_lp_burned: u64,
    pub vault: Pubkey,
    pub vault_asset_mint: Pubkey,
    pub vault_asset_total_value_unlocked_before: u64,
    pub vault_asset_total_value_before: u64,
    pub vault_asset_total_value_after: u64,
    pub vault_lp_supply_incl_fees_before: u64,
    pub vault_lp_supply_incl_fees_after: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub withdrawn_ts: u64,
}

#[event]
#[derive(Default)]
pub struct DepositStrategyEvent {
    pub manager: Pubkey,
    pub vault: Pubkey,
    pub strategy: Pubkey,
    pub strategy_init_receipt: Pubkey,
    pub adaptor_program: Pubkey,
    pub vault_asset_mint: Pubkey,
    pub vault_amount_asset_deposited: u64,
    pub vault_asset_total_value_before: u64,
    pub vault_asset_total_value_after: u64,
    pub vault_lp_supply_incl_fees_before: u64,
    pub vault_lp_supply_incl_fees_after: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub vault_asset_idle_ata_amount_before: u64,
    pub vault_asset_idle_ata_amount_after: u64,
    pub strategy_position_value_before: u64,
    pub strategy_position_value_after: u64,
    pub deposited_ts: u64,
}

#[event]
#[derive(Default)]
pub struct WithdrawStrategyEvent {
    pub manager: Pubkey,
    pub vault: Pubkey,
    pub strategy: Pubkey,
    pub strategy_init_receipt: Pubkey,
    pub adaptor_program: Pubkey,
    pub vault_asset_mint: Pubkey,
    pub vault_amount_asset_withdrawn: u64,
    pub vault_asset_total_value_before: u64,
    pub vault_asset_total_value_after: u64,
    pub vault_lp_supply_incl_fees_before: u64,
    pub vault_lp_supply_incl_fees_after: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub vault_asset_idle_ata_amount_before: u64,
    pub vault_asset_idle_ata_amount_after: u64,
    pub strategy_position_value_before: u64,
    pub strategy_position_value_after: u64,
    pub withdrawn_ts: u64,
}

#[event]
#[derive(Default)]
pub struct DirectWithdrawStrategyEvent {
    pub user: Pubkey,
    pub user_amount_asset_withdrawn: u64,
    pub user_amount_lp_burned: u64,
    pub vault: Pubkey,
    pub strategy: Pubkey,
    pub strategy_init_receipt: Pubkey,
    pub direct_withdraw_init_receipt: Pubkey,
    pub adaptor_program: Pubkey,
    pub vault_asset_mint: Pubkey,
    pub vault_asset_total_value_unlocked_before: u64,
    pub vault_asset_total_value_before: u64,
    pub vault_asset_total_value_after: u64,
    pub vault_lp_supply_incl_fees_before: u64,
    pub vault_lp_supply_incl_fees_after: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub strategy_position_value_before: u64,
    pub strategy_position_value_after: u64,
    pub withdrawn_ts: u64,
}

#[event]
#[derive(Default)]
pub struct RequestWithdrawVaultEvent {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub requested_amount: u64,
    pub is_amount_in_lp: bool,
    pub is_withdraw_all: bool,
    pub request_withdraw_vault_receipt: Pubkey,
    pub amount_lp_escrowed: u64,
    pub amount_asset_to_withdraw_decimal_bits: u128,
    pub withdrawable_from_ts: u64,
    pub vault_asset_mint: Pubkey,
    pub vault_asset_total_value_unlocked: u64,
    pub vault_asset_total_value: u64,
    pub vault_lp_supply_incl_fees: u64,
    pub requested_ts: u64,
}

#[event]
#[derive(Default)]
pub struct CancelRequestWithdrawVaultEvent {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub request_withdraw_vault_receipt: Pubkey,
    pub amount_lp_refunded: u64,
    pub amount_lp_burned: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub cancelled_ts: u64,
}

#[event]
#[derive(Default)]
pub struct HarvestFeeEvent {
    pub harvester: Pubkey,
    pub vault: Pubkey,
    pub protocol: Pubkey,
    pub admin: Pubkey,
    pub manager: Pubkey,
    pub amount_lp_admin_fees: u64,
    pub amount_lp_manager_fees: u64,
    pub amount_lp_protocol_fees: u64,
}

#[event]
#[derive(Default)]
pub struct RemoveAdaptorEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub adaptor_program: Pubkey,
    pub adaptor_add_receipt: Pubkey,
    pub removed_ts: u64,
}

#[event]
#[derive(Default)]
pub struct InitializeDirectWithdrawStrategyEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub strategy: Pubkey,
    pub strategy_init_receipt: Pubkey,
    pub adaptor_program: Pubkey,
    pub instruction_discriminator: Vec<u8>,
    pub additional_args: Vec<u8>,
    pub allow_user_args: bool,
    pub initialized_ts: u64,
}

#[event]
#[derive(Default)]
pub struct InitProtocolEvent {
    pub admin: Pubkey,
    pub operational_state: u16,
    pub initialized_ts: u64,
}

#[event]
#[derive(Default)]
pub struct UpdateProtocolEvent {
    pub protocol: Pubkey,
    pub admin_before: Pubkey,
    pub admin_after: Pubkey,
    pub operational_state_before: u16,
    pub operational_state_after: u16,
    pub updated_ts: u64,
}

#[event]
#[derive(Default)]
pub struct CloseStrategyEvent {
    pub manager: Pubkey,
    pub vault: Pubkey,
    pub strategy: Pubkey,
    pub strategy_init_receipt: Pubkey,
    pub closed_ts: u64,
}

#[event]
#[derive(Default)]
pub struct CalibrateHighWaterMarkEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub vault_asset_total_value: u64,
    pub vault_lp_supply_incl_fees_before: u64,
    pub vault_lp_supply_incl_fees_after: u64,
    pub vault_highest_asset_per_lp_decimal_bits_before: u128,
    pub vault_highest_asset_per_lp_decimal_bits_after: u128,
    pub amount_lp_burned: u64,
    pub calibrated_ts: u64,
}

#[event]
#[derive(Default)]
pub struct UpdateVaultConfigEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub updated_ts: u64,
}

#[event]
#[derive(Default)]
pub struct UpdateVaultProtocolFeeEvent {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub field: String,
    pub old_value: u16,
    pub new_value: u16,
    pub updated_ts: u64,
}
