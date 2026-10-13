use anchor_lang::prelude::*;
use instructions::*;
use state::*;

pub mod error;
pub mod instructions;
pub mod state;
#[cfg(test)]
pub mod tests;
pub mod utils;

declare_id!("vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8");

#[program]
pub mod voltr_vault {
    use super::*;

    pub fn init_or_update_protocol(
        ctx: Context<InitOrUpdateProtocol>,
        operational_state: u16,
    ) -> Result<()> {
        handler_init_or_update_protocol(ctx, operational_state)?;
        Ok(())
    }

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        config: VaultInitializationInput,
        name: String,
        description: String,
    ) -> Result<()> {
        handler_initialize_vault(ctx, config, name, description)?;
        Ok(())
    }

    pub fn deposit_vault(ctx: Context<DepositVault>, amount: u64) -> Result<()> {
        handler_deposit_vault(ctx, amount)?;
        Ok(())
    }

    pub fn request_withdraw_vault(
        ctx: Context<RequestWithdrawVault>,
        amount: u64,
        is_amount_in_lp: bool,
        is_withdraw_all: bool,
    ) -> Result<()> {
        handler_request_withdraw_vault(ctx, amount, is_amount_in_lp, is_withdraw_all)?;
        Ok(())
    }

    pub fn cancel_request_withdraw_vault(ctx: Context<CancelRequestWithdrawVault>) -> Result<()> {
        handler_cancel_request_withdraw_vault(ctx)?;
        Ok(())
    }

    pub fn withdraw_vault(ctx: Context<WithdrawVault>) -> Result<()> {
        handler_withdraw_vault(ctx)?;
        Ok(())
    }

    pub fn add_adaptor(ctx: Context<AddAdaptor>) -> Result<()> {
        handler_add_adaptor(ctx)?;
        Ok(())
    }

    pub fn remove_adaptor(ctx: Context<RemoveAdaptor>) -> Result<()> {
        handler_remove_adaptor(ctx)?;
        Ok(())
    }

    pub fn deposit_strategy<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, DepositStrategy<'info>>,
        amount: u64,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
    ) -> Result<()> {
        handler_deposit_strategy(ctx, amount, instruction_discriminator, additional_args)?;
        Ok(())
    }

    pub fn initialize_strategy<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitializeStrategy<'info>>,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
    ) -> Result<()> {
        handler_initialize_strategy(ctx, instruction_discriminator, additional_args)?;
        Ok(())
    }

    pub fn withdraw_strategy<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawStrategy<'info>>,
        amount: u64,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
    ) -> Result<()> {
        handler_withdraw_strategy(ctx, amount, instruction_discriminator, additional_args)?;
        Ok(())
    }

    pub fn initialize_direct_withdraw_strategy(
        ctx: Context<InitializeDirectWithdrawStrategy>,
        instruction_discriminator: Option<Vec<u8>>,
        additional_args: Option<Vec<u8>>,
        allow_user_args: bool,
    ) -> Result<()> {
        handler_initialize_direct_withdraw_strategy(
            ctx,
            instruction_discriminator,
            additional_args,
            allow_user_args,
        )?;
        Ok(())
    }

    pub fn direct_withdraw_strategy<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, DirectWithdrawStrategy<'info>>,
        user_args: Option<Vec<u8>>,
    ) -> Result<()> {
        handler_direct_withdraw_strategy(ctx, user_args)?;
        Ok(())
    }

    pub fn harvest_fee(ctx: Context<HarvestFee>) -> Result<()> {
        handler_harvest_fee(ctx)?;
        Ok(())
    }

    pub fn close_strategy(ctx: Context<CloseStrategy>) -> Result<()> {
        handler_close_strategy(ctx)?;
        Ok(())
    }

    pub fn create_lp_metadata(
        ctx: Context<CreateLpMetadata>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        handler_create_lp_metadata(ctx, name, symbol, uri)
    }

    pub fn calibrate_high_water_mark(ctx: Context<CalibrateHighWaterMark>) -> Result<()> {
        handler_calibrate_high_water_mark(ctx)?;
        Ok(())
    }

    pub fn update_vault_config(
        ctx: Context<UpdateVaultConfig>,
        field: VaultConfigField,
        data: Vec<u8>,
    ) -> Result<()> {
        handler_update_vault_config(ctx, field, data)?;
        Ok(())
    }

    pub fn update_vault_protocol_fee(
        ctx: Context<UpdateVaultProtocolFee>,
        field: ProtocolFeeField,
        fee_bps: u16,
    ) -> Result<()> {
        handler_update_vault_protocol_fee(ctx, field, fee_bps)?;
        Ok(())
    }
}
