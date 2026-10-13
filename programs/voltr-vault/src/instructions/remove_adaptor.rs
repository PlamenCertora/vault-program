use anchor_lang::prelude::*;

use crate::{
    state::RemoveAdaptorEvent,
    utils::constants::{ADAPTOR_ADD_RECEIPT_SEED, PROTOCOL_SEED},
    AdaptorAddReceipt, OperationalState, Protocol, Vault,
};

pub fn handler_remove_adaptor(ctx: Context<RemoveAdaptor>) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::REMOVE_ADAPTOR)?;

    emit!(RemoveAdaptorEvent {
        admin: ctx.accounts.admin.key(),
        vault: ctx.accounts.vault.key(),
        adaptor_program: ctx.accounts.adaptor_program.key(),
        adaptor_add_receipt: ctx.accounts.adaptor_add_receipt.key(),
        removed_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct RemoveAdaptor<'info> {
    #[account(mut)]
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
        seeds = [ADAPTOR_ADD_RECEIPT_SEED, vault.key().as_ref(), adaptor_program.key().as_ref()],
        bump = adaptor_add_receipt.load()?.bump,
        close = admin
    )]
    pub adaptor_add_receipt: AccountLoader<'info, AdaptorAddReceipt>,

    /// CHECK:
    pub adaptor_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
