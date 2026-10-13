use anchor_lang::prelude::*;

use crate::{
    error::VaultError,
    state::{InitProtocolEvent, UpdateProtocolEvent},
    utils::constants::PROTOCOL_SEED,
    Protocol,
};

pub fn handler_init_or_update_protocol(
    ctx: Context<InitOrUpdateProtocol>,
    operational_state: u16,
) -> Result<()> {
    ctx.accounts
        .init_or_update_protocol(operational_state, ctx.bumps.protocol)?;
    Ok(())
}

#[derive(Accounts)]
pub struct InitOrUpdateProtocol<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub current_admin: Signer<'info>,

    /// CHECK: new admin
    #[account()]
    pub new_admin: AccountInfo<'info>,

    #[account(
        init_if_needed,
        seeds = [PROTOCOL_SEED],
        bump,
        payer = payer,
        space = std::mem::size_of::<Protocol>() + 8
    )]
    pub protocol: AccountLoader<'info, Protocol>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitOrUpdateProtocol<'info> {
    fn init_or_update_protocol(&self, operational_state: u16, bump: u8) -> Result<()> {
        let is_not_initialized =
            Pubkey::try_from_slice(&self.protocol.to_account_info().data.borrow()[8..40])
                .map(|key| key == Pubkey::default())
                .unwrap_or(false);

        if is_not_initialized {
            require!(
                self.current_admin.key() == self.new_admin.key(),
                VaultError::InvalidAccountInput
            );
            let protocol = &mut self.protocol.load_init()?;
            protocol.init(self.new_admin.key(), operational_state, bump)?;

            emit!(InitProtocolEvent {
                admin: self.new_admin.key(),
                operational_state,
                initialized_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
            });
        } else {
            let protocol = &mut self.protocol.load_mut()?;
            let (admin_before, operational_state_before) =
                (protocol.admin, protocol.operational_state);
            require!(
                self.current_admin.key() == protocol.admin,
                VaultError::InvalidAccountInput
            );

            protocol.update(self.new_admin.key(), operational_state)?;
            emit!(UpdateProtocolEvent {
                protocol: self.protocol.key(),
                admin_before,
                admin_after: self.new_admin.key(),
                operational_state_before,
                operational_state_after: operational_state,
                updated_ts: u64::try_from(Clock::get()?.unix_timestamp)?,
            });
        }

        Ok(())
    }
}
