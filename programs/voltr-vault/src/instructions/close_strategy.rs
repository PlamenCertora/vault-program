use anchor_lang::prelude::*;

use crate::state::CloseStrategyEvent;
use crate::utils::constants::{PROTOCOL_SEED, STRATEGY_INIT_RECEIPT_SEED};
use crate::{OperationalState, Protocol, StrategyInitReceipt, Vault};

#[derive(Accounts)]
pub struct CloseStrategy<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub manager: Signer<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    /// CHECK: vault checked with strategy init and adaptor add receipt seeds
    #[account(
        constraint = vault.load()?.manager == manager.key() || vault.load()?.admin == manager.key()
    )]
    pub vault: AccountLoader<'info, Vault>,

    /// CHECK: strategy checked with strategy init receipt seeds
    pub strategy: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [STRATEGY_INIT_RECEIPT_SEED, vault.key().as_ref(), strategy.key().as_ref()],
        bump = strategy_init_receipt.load()?.bump,
        constraint = strategy_init_receipt.load()?.position_value == 0,
        close = payer
    )]
    pub strategy_init_receipt: AccountLoader<'info, StrategyInitReceipt>,

    pub system_program: Program<'info, System>,
}

pub fn handler_close_strategy(ctx: Context<CloseStrategy>) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::CLOSE_STRATEGY)?;

    emit!(CloseStrategyEvent {
        manager: ctx.accounts.manager.key(),
        vault: ctx.accounts.vault.key(),
        strategy: ctx.accounts.strategy.key(),
        strategy_init_receipt: ctx.accounts.strategy_init_receipt.key(),
        closed_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
    });

    Ok(())
}
