use anchor_lang::prelude::*;

use crate::{
    state::AddAdaptorEvent,
    utils::constants::{ADAPTOR_ADD_RECEIPT_SEED, PROTOCOL_SEED},
    AdaptorAddReceipt, OperationalState, Protocol, Vault,
};

pub fn handler_add_adaptor(ctx: Context<AddAdaptor>) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::ADD_ADAPTOR)?;

    let adaptor_add_receipt = &mut ctx.accounts.adaptor_add_receipt.load_init()?;
    adaptor_add_receipt.init(
        ctx.accounts.vault.key(),
        ctx.accounts.adaptor_program.key(),
        1,
        ctx.bumps.adaptor_add_receipt,
        ctx.accounts.vault.load()?.get_total_asset_value(),
        Clock::get()?.epoch,
    )?;

    emit!(AddAdaptorEvent {
        admin: ctx.accounts.admin.key(),
        vault: ctx.accounts.vault.key(),
        adaptor_program: ctx.accounts.adaptor_program.key(),
        adaptor_add_receipt: ctx.accounts.adaptor_add_receipt.key(),
        adaptor_last_updated_ts: adaptor_add_receipt.last_updated_epoch,
        added_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct AddAdaptor<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

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
        init,
        seeds = [ADAPTOR_ADD_RECEIPT_SEED, vault.key().as_ref(), adaptor_program.key().as_ref()],
        bump,
        payer = payer,
        space = std::mem::size_of::<AdaptorAddReceipt>() + 8
    )]
    pub adaptor_add_receipt: AccountLoader<'info, AdaptorAddReceipt>,

    /// CHECK:
    #[account(executable)]
    pub adaptor_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
