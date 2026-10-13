use anchor_lang::prelude::*;

use crate::{
    error::VaultError,
    utils::{
        calc_eligible_profit_with_decimal_bits, calc_fee_lp_to_burn, calc_fee_lp_to_mint,
        calc_fee_split, calc_init_lp_to_mint, calc_management_fee_amount_in_asset,
        calc_performance_fee_amount_in_asset,
        constants::{DEAD_WEIGHT, MAX_FEE_BPS},
        Decimal, SafeMath,
    },
};

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct Vault {
    /// The vault's name.
    pub name: [u8; 32],

    /// A description or summary for this vault.
    pub description: [u8; 64],

    /// The vault’s main asset configuration (inline nested struct).
    pub asset: VaultAsset,

    /// The vault’s LP (share) configuration (inline nested struct).
    pub lp: VaultLp,

    /// The manager of this vault (has certain permissions).
    pub manager: Pubkey,

    /// The admin of this vault (broader or fallback permissions).
    pub admin: Pubkey,

    /// The vault fee, cap, and locked profit degradation duration configuration (inline nested struct).
    pub vault_configuration: VaultConfiguration,

    /// The vault fee and cap configuration (inline nested struct).
    pub fee_configuration: FeeConfiguration,

    /// The fee update state of the vault.
    pub fee_update: FeeUpdate,

    /// The fee state of the vault.
    pub fee_state: FeeState,

    /// The accumulated dead weight of the vault.
    pub dead_weight: u64,

    pub high_water_mark: HighWaterMark,

    /// The last time (Unix timestamp) this vault data was updated.
    pub last_updated_ts: u64,

    /// The version of the vault.
    pub version: u8,

    /// padding to align future 8-byte fields on 8-byte boundaries.
    pub _padding0: [u8; 7],

    /// The locked profit state of the vault.
    pub locked_profit_state: LockedProfitState,

    /// Reserved bytes for future use.
    pub reserved: [u8; 240],
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            name: [0u8; 32],
            description: [0u8; 64],
            asset: VaultAsset::default(),
            lp: VaultLp::default(),
            manager: Pubkey::default(),
            admin: Pubkey::default(),
            vault_configuration: VaultConfiguration::default(),
            fee_configuration: FeeConfiguration::default(),
            fee_update: FeeUpdate::default(),
            fee_state: FeeState::default(),
            dead_weight: 0,
            high_water_mark: HighWaterMark::default(),
            last_updated_ts: 0,
            version: 0,
            _padding0: [0u8; 7],
            locked_profit_state: LockedProfitState::default(),
            reserved: [0u8; 240],
        }
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct LockedProfitState {
    pub last_updated_locked_profit: u64,
    pub last_report: u64,
}

impl LockedProfitState {
    pub fn calculate_locked_profit(
        &self,
        locked_profit_degradation_duration: u64,
        current_time: u64,
    ) -> Result<u64> {
        let duration = u128::from(current_time.safe_sub(self.last_report)?);
        let locked_profit_degradation_duration = u128::from(locked_profit_degradation_duration);
        // time has fully elapsed
        if duration > locked_profit_degradation_duration || locked_profit_degradation_duration == 0
        {
            return Ok(0);
        }

        // locked_profit = (locked_profit_degradation_duration - time_elapsed)/locked_profit_degradation_duration * last_updated_locked_profit
        let locked_profit = u128::from(self.last_updated_locked_profit)
            .safe_mul(u128::from(locked_profit_degradation_duration).safe_sub(duration)?)?
            .safe_div(u128::from(locked_profit_degradation_duration))?;

        Ok(u64::try_from(locked_profit)?)
    }

    pub fn update_last_report(
        &mut self,
        locked_profit_degradation_duration: u64,
        current_time: u64,
        profit: u64,
    ) -> Result<()> {
        let locked_profit =
            self.calculate_locked_profit(locked_profit_degradation_duration, current_time)?;
        self.last_updated_locked_profit = locked_profit.safe_add(profit)?;
        self.last_report = current_time;
        Ok(())
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct VaultAsset {
    /// The mint for the vault’s main asset.
    pub mint: Pubkey, // offset 0..32

    /// The “idle” token account holding un-invested assets.
    pub idle_ata: Pubkey, // offset 32..64

    /// The total amount of this asset currently in the vault.
    pub total_value: u64, // offset 64..72

    /// The bump for the vault asset mint.
    pub idle_ata_auth_bump: u8, // offset 72..73

    /// Reserved bytes for future use.
    pub reserved: [u8; 95], // offset 73..168
}

impl Default for VaultAsset {
    fn default() -> Self {
        Self {
            mint: Pubkey::default(),
            idle_ata: Pubkey::default(),
            total_value: 0,
            idle_ata_auth_bump: 0,
            reserved: [0u8; 95],
        }
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct VaultLp {
    /// The LP mint (e.g., representing shares in this vault).
    pub mint: Pubkey, // offset 0..32

    /// The bump for the vault LP mint.
    pub mint_bump: u8, // offset 64..65

    /// The bump for the vault LP mint authority.
    pub mint_auth_bump: u8, // offset 65..66

    /// Reserved bytes for future use.
    pub reserved: [u8; 62], // offset 67..160
}

impl Default for VaultLp {
    fn default() -> Self {
        Self {
            mint: Pubkey::default(),
            mint_bump: 0,
            mint_auth_bump: 0,
            reserved: [0u8; 62],
        }
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct VaultConfiguration {
    /// The maximum total amount allowed in the vault.
    pub max_cap: u64, // offset 0..8

    /// active from timestamp
    pub start_at_ts: u64, // offset 8..16

    /// The locked profit degradation duration.
    pub locked_profit_degradation_duration: u64, // offset 16..24

    /// The waiting period for a withdrawal. prec: seconds
    pub withdrawal_waiting_period: u64, // offset 24..32

    /// Reserved bytes for future use.
    pub disabled_operations: u16, // offset 32..34

    /// Reserved bytes for future use.
    pub reserved: [u8; 46], // offset 34..80
}

impl VaultConfiguration {
    pub fn check_operation_disabled(&self, operation: DisabledOperation) -> Result<()> {
        let current_flags = DisabledOperation::from_bits_truncate(self.disabled_operations);
        if current_flags.contains(operation) {
            return Err(VaultError::OperationNotAllowed.into());
        }

        Ok(())
    }
}

bitflags::bitflags! {
    pub struct DisabledOperation: u16 {
        const DEPOSIT_VAULT = 1 << 0;
        const WITHDRAW_VAULT = 1 << 1;
        const DIRECT_WITHDRAW = 1 << 2;
        const HARVEST_FEE = 1 << 3;
    }
}

impl Default for VaultConfiguration {
    fn default() -> Self {
        Self {
            max_cap: u64::MAX,
            start_at_ts: 0,
            locked_profit_degradation_duration: 0,
            withdrawal_waiting_period: 0,
            disabled_operations: 0,
            reserved: [0u8; 46],
        }
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct FeeConfiguration {
    /// Manager performance fee in basis points (BPS).
    pub manager_performance_fee: u16, // offset 0..2

    /// Admin performance fee in basis points (BPS).
    pub admin_performance_fee: u16, // offset 2..4

    /// Manager management fee in basis points (BPS).
    pub manager_management_fee: u16, // offset 4..6

    /// Admin management fee in basis points (BPS).
    pub admin_management_fee: u16, // offset 6..8

    /// The redemption fee in basis points (BPS).
    pub redemption_fee: u16, // offset 8..10

    /// The issuance fee in basis points (BPS).
    pub issuance_fee: u16, // offset 10..12

    /// Protocol performance fee in basis points (BPS).
    pub protocol_performance_fee: u16, // offset 12..14

    /// Protocol management fee in basis points (BPS).
    pub protocol_management_fee: u16, // offset 14..16

    /// Reserved bytes for future use.
    pub reserved: [u8; 32], // offset 16..48
}

impl Default for FeeConfiguration {
    fn default() -> Self {
        Self {
            manager_performance_fee: 0,
            admin_performance_fee: 0,
            manager_management_fee: 0,
            admin_management_fee: 0,
            redemption_fee: 0,
            issuance_fee: 0,
            protocol_performance_fee: 0,
            protocol_management_fee: 0,
            reserved: [0u8; 32],
        }
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct HighWaterMark {
    /// The highest recorded total asset value per share
    pub highest_asset_per_lp_decimal_bits: u128,
    /// The timestamp when the high water mark was last updated
    pub last_updated_ts: u64,
    /// Reserved for future use
    pub reserved: [u8; 8],
}

impl HighWaterMark {
    pub fn get_highest_asset_per_lp_decimal_bits(&self) -> Result<u128> {
        Ok(self.highest_asset_per_lp_decimal_bits)
    }

    pub fn calibrate_high_water_mark(
        &mut self,
        current_total_assets: u64,
        total_lp: u64,
        current_time: u64,
    ) -> Result<()> {
        if total_lp == 0 {
            return Ok(());
        }

        let current_asset_per_lp_decimal =
            Decimal::from_num(current_total_assets).safe_div(Decimal::from_num(total_lp))?;

        self.highest_asset_per_lp_decimal_bits = current_asset_per_lp_decimal.to_bits();
        self.last_updated_ts = current_time;

        Ok(())
    }

    pub fn update_high_water_mark_idempotent(
        &mut self,
        current_total_assets: u64,
        total_lp: u64,
        current_time: u64,
    ) -> Result<()> {
        if total_lp == 0 {
            return Ok(());
        }

        let current_asset_per_lp_decimal =
            Decimal::from_num(current_total_assets).safe_div(Decimal::from_num(total_lp))?;

        let highest_asset_per_lp_decimal =
            Decimal::from_bits(self.highest_asset_per_lp_decimal_bits);

        if current_asset_per_lp_decimal.gt(&highest_asset_per_lp_decimal) {
            self.highest_asset_per_lp_decimal_bits = current_asset_per_lp_decimal.to_bits();
            self.last_updated_ts = current_time;
        }

        Ok(())
    }

    pub fn calculate_eligible_profit(
        &self,
        current_total_assets: u64,
        total_lp: u64,
    ) -> Result<u64> {
        if total_lp == 0
            || self
                .highest_asset_per_lp_decimal_bits
                .eq(&Decimal::ZERO.to_bits())
        {
            return Ok(current_total_assets);
        }

        let current_asset_per_lp_decimal_bits = Decimal::from_num(current_total_assets)
            .safe_div(Decimal::from_num(total_lp))?
            .to_bits();

        calc_eligible_profit_with_decimal_bits(
            self.highest_asset_per_lp_decimal_bits,
            current_asset_per_lp_decimal_bits,
            total_lp,
        )
    }
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct FeeUpdate {
    /// The timestamp when the performance fees were last updated.
    pub last_performance_fee_update_ts: u64,

    /// The timestamp when the management fees were last updated.
    pub last_management_fee_update_ts: u64,
}

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct FeeState {
    /// The accumulated manager fees in the vault.
    pub accumulated_lp_manager_fees: u64,

    /// The accumulated admin fees in the vault.
    pub accumulated_lp_admin_fees: u64,

    /// The accumulated protocol fees in the vault.
    pub accumulated_lp_protocol_fees: u64,

    /// Reserved bytes for future use.
    pub reserved: [u8; 16],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct UpdateVaultProtocolFeeInput {
    /// Protocol performance fee in basis points (BPS).
    pub protocol_performance_fee: u16,
    /// Protocol management fee in basis points (BPS).
    pub protocol_management_fee: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct VaultInitializationInput {
    /// The maximum total amount allowed in the vault.
    pub max_cap: u64,
    /// active from timestamp
    pub start_at_ts: u64,
    /// Manager performance fee in basis points (BPS).
    pub manager_performance_fee: u16,
    /// Admin performance fee in basis points (BPS).
    pub admin_performance_fee: u16,
    /// Manager management fee in basis points (BPS).
    pub manager_management_fee: u16,
    /// Admin management fee in basis points (BPS).
    pub admin_management_fee: u16,
    /// The locked profit degradation duration.
    pub locked_profit_degradation_duration: u64,
    /// The redemption fee in basis points (BPS).
    pub redemption_fee: u16,
    /// The issuance fee in basis points (BPS).
    pub issuance_fee: u16,
    /// The waiting period for a withdrawal.
    pub withdrawal_waiting_period: u64,
}

impl VaultInitializationInput {
    pub fn validate_fees(&self) -> Result<()> {
        require!(
            self.manager_performance_fee
                .safe_add(self.admin_performance_fee)?
                <= MAX_FEE_BPS,
            VaultError::InvalidFeeConfiguration
        );
        require!(
            self.redemption_fee <= MAX_FEE_BPS,
            VaultError::InvalidFeeConfiguration
        );
        require!(
            self.issuance_fee <= MAX_FEE_BPS,
            VaultError::InvalidFeeConfiguration
        );
        require!(
            self.manager_management_fee
                .safe_add(self.admin_management_fee)?
                <= MAX_FEE_BPS,
            VaultError::InvalidFeeConfiguration
        );

        Ok(())
    }
}

impl From<VaultInitializationInput> for VaultConfiguration {
    fn from(input: VaultInitializationInput) -> Self {
        Self {
            max_cap: input.max_cap,
            start_at_ts: input.start_at_ts,
            locked_profit_degradation_duration: input.locked_profit_degradation_duration,
            withdrawal_waiting_period: input.withdrawal_waiting_period,
            disabled_operations: DisabledOperation::empty().bits(),
            reserved: [0u8; 46],
        }
    }
}

impl From<VaultInitializationInput> for FeeConfiguration {
    fn from(input: VaultInitializationInput) -> Self {
        Self {
            manager_performance_fee: input.manager_performance_fee,
            admin_performance_fee: input.admin_performance_fee,
            manager_management_fee: input.manager_management_fee,
            admin_management_fee: input.admin_management_fee,
            redemption_fee: input.redemption_fee,
            issuance_fee: input.issuance_fee,
            protocol_performance_fee: 0,
            protocol_management_fee: 0,
            reserved: [0u8; 32],
        }
    }
}

impl Vault {
    pub fn get_total_fee_configuration_performance_fee(&self) -> Result<u16> {
        Ok(self
            .fee_configuration
            .admin_performance_fee
            .safe_add(self.fee_configuration.manager_performance_fee)?
            .safe_add(self.fee_configuration.protocol_performance_fee)?)
    }

    pub fn get_total_fee_configuration_management_fee(&self) -> Result<u16> {
        Ok(self
            .fee_configuration
            .admin_management_fee
            .safe_add(self.fee_configuration.manager_management_fee)?
            .safe_add(self.fee_configuration.protocol_management_fee)?)
    }

    pub fn calculate_locked_profit(&self, current_time: u64) -> Result<u64> {
        self.locked_profit_state.calculate_locked_profit(
            self.vault_configuration.locked_profit_degradation_duration,
            current_time,
        )
    }

    pub fn get_total_asset_value(&self) -> u64 {
        self.asset.total_value
    }

    pub fn get_unlocked_asset_value(&self, current_time: u64) -> Result<u64> {
        self.get_total_asset_value()
            .safe_sub(self.calculate_locked_profit(current_time)?)
    }

    pub fn decrement_total_value(&mut self, loss: u64) -> Result<()> {
        self.asset.total_value = self.asset.total_value.safe_sub(loss)?;
        Ok(())
    }

    pub fn increment_total_value_and_locked_profit(
        &mut self,
        current_time: u64,
        profit: u64,
    ) -> Result<()> {
        self.locked_profit_state.update_last_report(
            self.vault_configuration.locked_profit_degradation_duration,
            current_time,
            profit,
        )?;
        self.increment_total_value(profit)?;
        Ok(())
    }

    pub fn increment_total_value(&mut self, profit: u64) -> Result<()> {
        self.asset.total_value = self.asset.total_value.safe_add(profit)?;
        Ok(())
    }

    pub fn update_last_updated_ts(&mut self, current_ts: u64) -> Result<()> {
        self.last_updated_ts = current_ts;
        Ok(())
    }

    pub fn increment_admin_fees(&mut self, amount: u64) -> Result<()> {
        self.fee_state.accumulated_lp_admin_fees =
            self.fee_state.accumulated_lp_admin_fees.safe_add(amount)?;
        Ok(())
    }

    pub fn increment_manager_fees(&mut self, amount: u64) -> Result<()> {
        self.fee_state.accumulated_lp_manager_fees = self
            .fee_state
            .accumulated_lp_manager_fees
            .safe_add(amount)?;
        Ok(())
    }

    pub fn increment_protocol_fees(&mut self, amount: u64) -> Result<()> {
        self.fee_state.accumulated_lp_protocol_fees = self
            .fee_state
            .accumulated_lp_protocol_fees
            .safe_add(amount)?;
        Ok(())
    }

    pub fn reset_fee_state(&mut self) -> Result<()> {
        self.fee_state = FeeState {
            accumulated_lp_admin_fees: 0,
            accumulated_lp_manager_fees: 0,
            accumulated_lp_protocol_fees: 0,
            reserved: [0u8; 16],
        };
        Ok(())
    }

    pub fn get_total_accumulated_lp_fees(&self) -> Result<u64> {
        Ok(self
            .fee_state
            .accumulated_lp_admin_fees
            .safe_add(self.fee_state.accumulated_lp_manager_fees)?
            .safe_add(self.fee_state.accumulated_lp_protocol_fees)?)
    }

    pub fn get_total_lp_supply_incl_fees(&self, total_lp_supply_excl_fees: u64) -> Result<u64> {
        self.get_total_accumulated_lp_fees()?
            .safe_add(total_lp_supply_excl_fees)?
            .safe_add(self.dead_weight)
    }

    pub fn calibrate_high_water_mark(
        &mut self,
        total_lp_supply_excl_fees: u64,
        current_ts: u64,
    ) -> Result<(u64, u64, u128, u128, u64)> {
        // this hwm is greater than the current hwm
        let highest_asset_per_lp_decimal_bits_before = self
            .high_water_mark
            .get_highest_asset_per_lp_decimal_bits()?;

        let current_total_assets = self.get_total_asset_value();
        let total_lp_supply_incl_fees =
            self.get_total_lp_supply_incl_fees(total_lp_supply_excl_fees)?;

        let current_asset_per_lp_decimal_bits = Decimal::from_num(current_total_assets)
            .safe_div(Decimal::from_num(total_lp_supply_incl_fees))?
            .to_bits();

        // reverse the usual order to calculate profit
        let reverted_profit = calc_eligible_profit_with_decimal_bits(
            current_asset_per_lp_decimal_bits,
            highest_asset_per_lp_decimal_bits_before,
            total_lp_supply_incl_fees,
        )?;

        let performance_fee_bps = self.get_total_fee_configuration_performance_fee()?;
        let fee_amount_in_asset =
            calc_performance_fee_amount_in_asset(reverted_profit, performance_fee_bps)?;

        let lp_to_burn = if current_total_assets == 0 {
            0
        } else {
            calc_fee_lp_to_burn(
                fee_amount_in_asset,
                current_total_assets,
                total_lp_supply_incl_fees,
            )?
        };

        self.high_water_mark.calibrate_high_water_mark(
            current_total_assets,
            total_lp_supply_incl_fees.safe_sub(lp_to_burn)?,
            current_ts,
        )?;

        let highest_asset_per_lp_decimal_bits_after = self
            .high_water_mark
            .get_highest_asset_per_lp_decimal_bits()?;

        Ok((
            current_total_assets,
            total_lp_supply_incl_fees,
            highest_asset_per_lp_decimal_bits_before,
            highest_asset_per_lp_decimal_bits_after,
            lp_to_burn,
        ))
    }

    pub fn update_high_water_mark_idempotent(
        &mut self,
        total_lp_supply_excl_fees: u64,
        current_ts: u64,
    ) -> Result<()> {
        let current_total_assets = self.get_total_asset_value();
        let total_lp_supply_incl_fees =
            self.get_total_lp_supply_incl_fees(total_lp_supply_excl_fees)?;
        self.high_water_mark.update_high_water_mark_idempotent(
            current_total_assets,
            total_lp_supply_incl_fees,
            current_ts,
        )
    }

    pub fn handle_accumulate_fee_lp(
        &mut self,
        lp_to_mint: u64,
        protocol_fee_bps: u16,
        admin_fee_bps: u16,
        manager_fee_bps: u16,
    ) -> Result<()> {
        if lp_to_mint == 0 {
            return Ok(());
        }

        let (admin_lp, manager_lp, protocol_lp) =
            calc_fee_split(lp_to_mint, protocol_fee_bps, admin_fee_bps, manager_fee_bps)?;

        self.increment_admin_fees(admin_lp)?;
        self.increment_manager_fees(manager_lp)?;
        self.increment_protocol_fees(protocol_lp)?;

        Ok(())
    }

    pub fn handle_management_fee(
        &mut self,
        current_ts: u64,
        total_asset_value: u64,
        total_lp_supply_incl_fees: u64, // Pass the current total LP supply (including already accrued fees)
    ) -> Result<()> {
        let management_fee_bps = self.get_total_fee_configuration_management_fee()?;
        if self.fee_update.last_management_fee_update_ts == 0 || // v2 vaults
            total_asset_value == 0 ||
            management_fee_bps == 0
        {
            self.fee_update.last_management_fee_update_ts = current_ts;
            return Ok(());
        }

        let time_elapsed = current_ts.safe_sub(self.fee_update.last_management_fee_update_ts)?;
        if time_elapsed == 0 {
            return Ok(());
        }

        // Calculate Manager Fee Value
        let fee_amount_in_asset = calc_management_fee_amount_in_asset(
            time_elapsed,
            total_asset_value,
            management_fee_bps,
        )?;

        if fee_amount_in_asset.eq(&0) {
            self.fee_update.last_management_fee_update_ts = current_ts;
            return Ok(());
        }

        if fee_amount_in_asset.ge(&total_asset_value) {
            return Err(VaultError::FeeExceedsTotalAssetValue.into());
        }

        let total_lp_to_mint = calc_fee_lp_to_mint(
            fee_amount_in_asset,
            total_lp_supply_incl_fees,
            total_asset_value,
        )?;

        self.handle_accumulate_fee_lp(
            total_lp_to_mint,
            self.fee_configuration.protocol_management_fee,
            self.fee_configuration.admin_management_fee,
            self.fee_configuration.manager_management_fee,
        )?;

        self.fee_update.last_management_fee_update_ts = current_ts;

        Ok(())
    }

    pub fn handle_performance_fee_and_update_vault<'info>(
        &mut self,
        is_profit: bool,
        pnl_amount: u64,
        vault_asset_mint_decimals: u8,
        vault_lp_mint_supply: u64,
        vault_lp_mint_decimals: u8,
        current_ts: u64,
    ) -> Result<()> {
        // 1) If profit is zero or we are not in a profit scenario, we will skip this.
        if is_profit && pnl_amount > 0 {
            self.increment_total_value_and_locked_profit(current_ts, pnl_amount)?;

            let total_assets = self.get_total_asset_value();
            let total_supply_with_fees =
                self.get_total_lp_supply_incl_fees(vault_lp_mint_supply)?;

            let eligible_profit = self
                .high_water_mark
                .calculate_eligible_profit(total_assets, total_supply_with_fees)?;

            if eligible_profit > 0 {
                // 2) Calculate fee BPS = admin + manager performance fee
                self.fee_update.last_performance_fee_update_ts = current_ts;

                let performance_fee_bps = self.get_total_fee_configuration_performance_fee()?;

                // 3) Compute fee_amount = (eligible_profit * performance_fee_bps) / 10000
                let fee_amount_in_asset =
                    calc_performance_fee_amount_in_asset(eligible_profit, performance_fee_bps)?;

                if fee_amount_in_asset == 0 {
                    // If the fee amount is zero, no need to mint anything
                    return Ok(());
                }

                // 4) Compute how many LP tokens to mint to the fee receiver
                //    proportion = fee_amount / total_assets * total_supply

                let lp_tokens_to_mint = if total_assets == 0 || total_supply_with_fees == 0 {
                    calc_init_lp_to_mint(
                        fee_amount_in_asset,
                        vault_asset_mint_decimals,
                        vault_lp_mint_decimals,
                    )?
                } else {
                    calc_fee_lp_to_mint(fee_amount_in_asset, total_supply_with_fees, total_assets)?
                };

                self.handle_accumulate_fee_lp(
                    lp_tokens_to_mint,
                    self.fee_configuration.protocol_performance_fee,
                    self.fee_configuration.admin_performance_fee,
                    self.fee_configuration.manager_performance_fee,
                )?;
            }
        } else {
            self.decrement_total_value(pnl_amount)?;
        }

        Ok(())
    }

    // should only charge once per vault
    pub fn handle_dead_weight_attribution_idempotent(
        &mut self,
        lp_to_mint_before_dead_weight: u64,
    ) -> Result<u64> {
        if self.dead_weight > 0 {
            Ok(lp_to_mint_before_dead_weight)
        } else {
            require!(
                lp_to_mint_before_dead_weight.ge(&DEAD_WEIGHT),
                VaultError::InvalidAmount
            );
            self.dead_weight = DEAD_WEIGHT;
            lp_to_mint_before_dead_weight.safe_sub(DEAD_WEIGHT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_implementations() {
        let vault = Vault::default();
        assert_eq!(vault.asset.total_value, 0);
        assert_eq!(vault.version, 0);

        let locked_profit = LockedProfitState::default();
        assert_eq!(locked_profit.last_updated_locked_profit, 0);

        let hwm = HighWaterMark::default();
        assert_eq!(hwm.highest_asset_per_lp_decimal_bits, 0);
    }

    #[test]
    fn test_basic_getters() {
        let mut vault = Vault::default();
        vault.asset.total_value = 1000;
        assert_eq!(vault.get_total_asset_value(), 1000);
    }
}
