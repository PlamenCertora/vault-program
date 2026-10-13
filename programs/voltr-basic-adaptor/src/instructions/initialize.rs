use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK:
    pub strategy: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(_ctx: Context<Initialize>) -> Result<()> {
    Ok(())
}
