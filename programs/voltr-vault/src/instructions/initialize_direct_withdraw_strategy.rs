use anchor_lang::prelude::*;

use crate::{
    state::InitializeDirectWithdrawStrategyEvent,
    utils::constants::{
        ADAPTOR_ADD_RECEIPT_SEED, DIRECT_WITHDRAW_INIT_RECEIPT_SEED, PROTOCOL_SEED,
        STRATEGY_INIT_RECEIPT_SEED,
    },
    AdaptorAddReceipt, DirectWithdrawInitReceipt, OperationalState, Protocol, StrategyInitReceipt,
    Vault,
};

pub fn handler_initialize_direct_withdraw_strategy(
    ctx: Context<InitializeDirectWithdrawStrategy>,
    instruction_discriminator: Option<Vec<u8>>,
    additional_args: Option<Vec<u8>>,
    allow_user_args: bool,
) -> Result<()> {
    ctx.accounts
        .adaptor_add_receipt
        .load()?
        .check_adaptor_validity(Clock::get()?.epoch)?;
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::INIT_DIRECT_WITHDRAW)?;

    let direct_withdraw_init_receipt = &mut ctx.accounts.direct_withdraw_init_receipt;
    direct_withdraw_init_receipt.init(
        ctx.accounts.vault.key(),
        ctx.accounts.strategy_init_receipt.load()?.strategy,
        ctx.accounts.adaptor_add_receipt.load()?.adaptor_program,
        instruction_discriminator.clone(),
        additional_args.clone(),
        allow_user_args,
        0,
        ctx.bumps.direct_withdraw_init_receipt,
    );

    emit!(InitializeDirectWithdrawStrategyEvent {
        admin: ctx.accounts.admin.key(),
        vault: ctx.accounts.vault.key(),
        strategy: ctx.accounts.strategy_init_receipt.load()?.strategy,
        strategy_init_receipt: ctx.accounts.direct_withdraw_init_receipt.key(),
        adaptor_program: ctx.accounts.adaptor_add_receipt.load()?.adaptor_program,
        instruction_discriminator: instruction_discriminator.unwrap_or_default(),
        additional_args: additional_args.unwrap_or_default(),
        allow_user_args,
        initialized_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(instruction_discriminator: Option<Vec<u8>>, additional_args: Option<Vec<u8>>)]
pub struct InitializeDirectWithdrawStrategy<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub admin: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    /// CHECK: vault checked with strategy init receipt seeds
    #[account(
        mut,
        has_one = admin
    )]
    pub vault: AccountLoader<'info, Vault>,

    /// CHECK: strategy checked with strategy init receipt seeds
    pub strategy: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            STRATEGY_INIT_RECEIPT_SEED,
            vault.key().as_ref(),
            strategy.key().as_ref(),
        ],
        bump = strategy_init_receipt.load()?.bump
    )]
    pub strategy_init_receipt: AccountLoader<'info, StrategyInitReceipt>,

    #[account(
        seeds = [ADAPTOR_ADD_RECEIPT_SEED, vault.key().as_ref(), adaptor_program.key().as_ref()],
        bump = adaptor_add_receipt.load()?.bump
    )]
    pub adaptor_add_receipt: AccountLoader<'info, AdaptorAddReceipt>,

    #[account(
        init,
        payer = payer,
        space = 8 + DirectWithdrawInitReceipt::len(instruction_discriminator, additional_args),
        seeds = [DIRECT_WITHDRAW_INIT_RECEIPT_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump
    )]
    pub direct_withdraw_init_receipt: Account<'info, DirectWithdrawInitReceipt>,

    /// CHECK: adaptor program checked with adaptor add receipt seeds
    #[account(address = strategy_init_receipt.load()?.adaptor_program)]
    pub adaptor_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
