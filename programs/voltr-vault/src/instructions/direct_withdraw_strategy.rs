use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{
    token::Token,
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    error::VaultError,
    state::{DirectWithdrawStrategyEvent, DisabledOperation, Vault},
    utils::{
        calc_withdraw_pnl,
        constants::{
            ADAPTOR_ADD_RECEIPT_SEED, DIRECT_WITHDRAW_INIT_RECEIPT_SEED, PROTOCOL_SEED,
            REQUEST_WITHDRAW_VAULT_RECEIPT_SEED, STRATEGY_INIT_RECEIPT_SEED, VAULT_LP_MINT_SEED,
            VAULT_STRATEGY_AUTH_SEED,
        },
        encode_additional_args, get_capped_withdraw_asset_to_redeem, get_return_u64,
        to_account_meta_modify_manager_non_signer_option, SafeMath,
    },
    AdaptorAddReceipt, DirectWithdrawInitReceipt, OperationalState, Protocol,
    RequestWithdrawVaultReceipt, StrategyInitReceipt,
};

pub fn handler_direct_withdraw_strategy<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, DirectWithdrawStrategy<'info>>,
    user_args: Option<Vec<u8>>,
) -> Result<()> {
    ctx.accounts.protocol.load()?.check_operation_state(
        OperationalState::DIRECT_WITHDRAW
            | OperationalState::WITHDRAW_VAULT
            | OperationalState::WITHDRAW_STRATEGY,
    )?;
    ctx.accounts
        .vault
        .load()?
        .vault_configuration
        .check_operation_disabled(
            DisabledOperation::DIRECT_WITHDRAW | DisabledOperation::WITHDRAW_VAULT,
        )?;
    ctx.accounts
        .adaptor_add_receipt
        .load()?
        .check_adaptor_validity(Clock::get()?.epoch)?;

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    require!(
        ctx.accounts
            .request_withdraw_vault_receipt
            .load()?
            .is_withdrawable(current_ts)?,
        VaultError::WithdrawalNotYetAvailable
    );

    // Validate inputs
    let total_lp_supply = ctx.accounts.vault_lp_mint.supply;
    require!(total_lp_supply > 0, VaultError::InvalidAmount);

    let (
        total_asset_value_before,
        total_unlocked_asset,
        total_lp_supply_incl_fees_before,
        vault_highest_asset_per_lp_decimal_bits_before,
        redemption_fee,
    ) = {
        let vault = &ctx.accounts.vault.load()?;
        (
            vault.get_total_asset_value(),
            vault.get_unlocked_asset_value(current_ts)?,
            vault.get_total_lp_supply_incl_fees(total_lp_supply)?,
            vault
                .high_water_mark
                .get_highest_asset_per_lp_decimal_bits()?,
            vault.fee_configuration.redemption_fee,
        )
    };
    require!(total_unlocked_asset > 0, VaultError::InvalidAmount);

    let (
        amount_lp_to_burn,
        initial_amount_asset_to_withdraw_decimal_bits,
        request_withdraw_vault_receipt_bump,
    ) = {
        let receipt = ctx.accounts.request_withdraw_vault_receipt.load()?;
        (
            receipt.amount_lp_escrowed,
            receipt.amount_asset_to_withdraw_decimal_bits,
            receipt.bump,
        )
    };

    let vault = &mut ctx.accounts.vault.load_mut()?;
    vault.handle_management_fee(
        current_ts,
        total_asset_value_before,
        total_lp_supply_incl_fees_before,
    )?;

    let total_lp_supply_incl_fees_after_management_fee =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    let amount_asset_to_withdraw = get_capped_withdraw_asset_to_redeem(
        initial_amount_asset_to_withdraw_decimal_bits,
        amount_lp_to_burn,
        total_lp_supply_incl_fees_after_management_fee,
        total_unlocked_asset,
        redemption_fee,
    )?;

    let vault_strategy_auth_bump = ctx
        .accounts
        .strategy_init_receipt
        .load()?
        .vault_strategy_auth_bump;

    let pre_invoke_strategy_ata_amount = ctx.accounts.vault_strategy_asset_ata.amount;
    let post_invoke_strategy_position_value = ctx
        .accounts
        .invoke_withdraw_strategy_cpi_get_position_value(
            ctx.remaining_accounts,
            amount_asset_to_withdraw,
            vault_strategy_auth_bump,
            user_args,
        )?;
    ctx.accounts.vault_strategy_asset_ata.reload()?;
    let post_invoke_strategy_ata_amount = ctx.accounts.vault_strategy_asset_ata.amount;
    let withdrawn_amount_to_strategy =
        post_invoke_strategy_ata_amount.safe_sub(pre_invoke_strategy_ata_amount)?;

    // only allow for rounding threshold
    require!(
        amount_asset_to_withdraw >= withdrawn_amount_to_strategy
            && amount_asset_to_withdraw.safe_sub(1)? <= withdrawn_amount_to_strategy,
        VaultError::InvalidAmount
    );

    ctx.accounts
        .burn_lp_from_receipt(amount_lp_to_burn, request_withdraw_vault_receipt_bump)?;

    ctx.accounts
        .transfer_strategy_to_user(withdrawn_amount_to_strategy)?;

    let current_ts = u64::try_from(Clock::get()?.unix_timestamp)?;

    let strategy_init_receipt = &mut ctx.accounts.strategy_init_receipt.load_mut()?;

    let pre_invoked_strategy_position_value = strategy_init_receipt.position_value;

    strategy_init_receipt.set_position_value(post_invoke_strategy_position_value, current_ts)?;

    let (is_profit, pnl_amount) = calc_withdraw_pnl(
        pre_invoked_strategy_position_value,
        post_invoke_strategy_position_value,
        withdrawn_amount_to_strategy,
    )?;

    // Call shared function for fee handling
    vault.handle_performance_fee_and_update_vault(
        is_profit,
        pnl_amount,
        ctx.accounts.vault_asset_mint.decimals,
        ctx.accounts.vault_lp_mint.supply,
        ctx.accounts.vault_lp_mint.decimals,
        current_ts,
    )?;

    // Update the vault
    ctx.accounts.vault_lp_mint.reload()?;
    vault.decrement_total_value(withdrawn_amount_to_strategy)?;
    vault.update_high_water_mark_idempotent(ctx.accounts.vault_lp_mint.supply, current_ts)?;
    vault.update_last_updated_ts(current_ts)?;

    emit!(DirectWithdrawStrategyEvent {
        user: ctx.accounts.user_transfer_authority.key(),
        user_amount_asset_withdrawn: amount_asset_to_withdraw,
        user_amount_lp_burned: amount_lp_to_burn,
        vault: ctx.accounts.vault.key(),
        strategy: ctx.accounts.strategy.key(),
        strategy_init_receipt: ctx.accounts.strategy_init_receipt.key(),
        direct_withdraw_init_receipt: ctx.accounts.direct_withdraw_init_receipt.key(),
        adaptor_program: ctx.accounts.adaptor_program.key(),
        vault_asset_mint: ctx.accounts.vault_asset_mint.key(),
        vault_asset_total_value_unlocked_before: total_unlocked_asset,
        vault_asset_total_value_before: total_asset_value_before,
        vault_asset_total_value_after: vault.get_total_asset_value(),
        vault_lp_supply_incl_fees_before: total_lp_supply_incl_fees_before,
        vault_lp_supply_incl_fees_after: vault
            .get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?,
        vault_highest_asset_per_lp_decimal_bits_before,
        vault_highest_asset_per_lp_decimal_bits_after: vault
            .high_water_mark
            .get_highest_asset_per_lp_decimal_bits()?,
        strategy_position_value_before: pre_invoked_strategy_position_value,
        strategy_position_value_after: post_invoke_strategy_position_value,
        withdrawn_ts: vault.last_updated_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct DirectWithdrawStrategy<'info> {
    /// The authority that owns the LP tokens and wants to redeem them
    #[account(mut)]
    pub user_transfer_authority: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(
        mut,
        constraint = vault.load()?.asset.mint == vault_asset_mint.key()
    )]
    pub vault: AccountLoader<'info, Vault>,

    #[account(
        seeds = [ADAPTOR_ADD_RECEIPT_SEED, vault.key().as_ref(), adaptor_program.key().as_ref()],
        bump = adaptor_add_receipt.load()?.bump
    )]
    pub adaptor_add_receipt: AccountLoader<'info, AdaptorAddReceipt>,

    #[account(
        mut,
        seeds = [STRATEGY_INIT_RECEIPT_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump = strategy_init_receipt.load()?.bump
    )]
    pub strategy_init_receipt: AccountLoader<'info, StrategyInitReceipt>,

    #[account(
        seeds = [DIRECT_WITHDRAW_INIT_RECEIPT_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump = direct_withdraw_init_receipt.bump
    )]
    pub direct_withdraw_init_receipt: Account<'info, DirectWithdrawInitReceipt>,

    /// CHECK: strategy checked with strategy init receipt seeds
    pub strategy: AccountInfo<'info>,

    #[account(mut, mint::token_program = asset_token_program)]
    pub vault_asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: InterfaceAccount<'info, Mint>,

    /// The request's LP token account from which LP tokens will be burned.
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
        seeds = [VAULT_STRATEGY_AUTH_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump = strategy_init_receipt.load()?.vault_strategy_auth_bump
    )]
    pub vault_strategy_auth: AccountInfo<'info>,

    /// The user's asset ATA to which asset tokens will be sent.
    #[account(
        mut,
        associated_token::mint = vault_asset_mint,
        associated_token::authority = user_transfer_authority,
        associated_token::token_program = asset_token_program,
    )]
    pub user_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(
            mut,
            associated_token::mint = vault_asset_mint,
            associated_token::authority = vault_strategy_auth,
            associated_token::token_program = asset_token_program,
        )]
    pub vault_strategy_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,

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

    /// CHECK: adaptor program checked with adaptor add receipt seeds
    #[account(address = direct_withdraw_init_receipt.adaptor_program)]
    pub adaptor_program: AccountInfo<'info>,

    pub asset_token_program: Interface<'info, TokenInterface>,
    pub lp_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> DirectWithdrawStrategy<'info> {
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

    /// Transfers assets from vault strategy ATA to user ATA
    pub fn transfer_strategy_to_user(&self, amount: u64) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.asset_token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.vault_strategy_asset_ata.to_account_info(),
                    to: self.user_asset_ata.to_account_info(),
                    authority: self.vault_strategy_auth.to_account_info(),
                    mint: self.vault_asset_mint.to_account_info(),
                },
                &[&[
                    VAULT_STRATEGY_AUTH_SEED,
                    self.vault.key().as_ref(),
                    self.strategy.key().as_ref(),
                    &[self.strategy_init_receipt.load()?.vault_strategy_auth_bump],
                ]],
            ),
            amount,
            self.vault_asset_mint.decimals,
        )
    }

    /// Builds and invokes the withdraw strategy CPI instruction
    pub fn invoke_withdraw_strategy_cpi_get_position_value(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        amount: u64,
        vault_strategy_auth_bump: u8,
        user_args: Option<Vec<u8>>,
    ) -> Result<u64> {
        let mut data = self
            .direct_withdraw_init_receipt
            .instruction_discriminator
            .clone();
        data.extend_from_slice(&amount.to_le_bytes());
        // TODO: to pass on raw user args
        encode_additional_args(
            &mut data,
            &self.direct_withdraw_init_receipt.additional_args,
        );
        if self.direct_withdraw_init_receipt.allow_user_args {
            encode_additional_args(&mut data, &user_args);
        }

        let mut cpi_accounts = vec![
            AccountMeta::new(self.vault_strategy_auth.key(), true),
            AccountMeta::new_readonly(self.strategy.key(), false),
            AccountMeta::new(self.vault_asset_mint.key(), false),
            AccountMeta::new(self.vault_strategy_asset_ata.key(), false),
            AccountMeta::new_readonly(self.asset_token_program.key(), false),
        ];
        cpi_accounts.extend(
            remaining_accounts
                .iter()
                .map(|acc| to_account_meta_modify_manager_non_signer_option(acc, None)),
        );

        let instruction = Instruction {
            program_id: self.adaptor_program.key(),
            accounts: cpi_accounts,
            data,
        };

        let mut account_infos = vec![
            self.vault_strategy_auth.clone(),
            self.strategy.clone(),
            self.vault_asset_mint.to_account_info(),
            self.vault_strategy_asset_ata.to_account_info(),
            self.asset_token_program.to_account_info(),
        ];

        account_infos.extend(remaining_accounts.iter().map(|acc| acc.to_account_info()));

        solana_invoke::invoke_signed(
            &instruction,
            &account_infos,
            &[&[
                VAULT_STRATEGY_AUTH_SEED,
                self.vault.key().as_ref(),
                self.strategy.key().as_ref(),
                &[vault_strategy_auth_bump],
            ]],
        )?;

        get_return_u64(instruction.program_id)
    }
}
