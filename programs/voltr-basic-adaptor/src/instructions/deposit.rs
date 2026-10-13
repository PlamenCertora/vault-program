use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositParams {
    pub placeholder_vec_len: u32,
    pub end_value: u64,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK:
    #[account(
        seeds = [b"strategy"],
        bump
    )]
    pub strategy: AccountInfo<'info>,

    #[account(mut)]
    pub vault_asset_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = vault_asset_mint,
        associated_token::authority = authority,
        associated_token::token_program = token_program,
    )]
    pub authority_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(
        mut,
        associated_token::mint = vault_asset_mint,
        associated_token::authority = strategy,
        associated_token::token_program = token_program,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
}

pub fn deposit_handler(
    ctx: Context<Deposit>,
    amount: u64,
    params: Option<DepositParams>,
) -> Result<u64> {
    if amount.gt(&0) {
        token_2022::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: ctx.accounts.authority_token_account.to_account_info(),
                    mint: ctx.accounts.vault_asset_mint.to_account_info(),
                    to: ctx.accounts.destination_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.vault_asset_mint.decimals,
        )?;
        ctx.accounts.destination_token_account.reload()?;
    }
    let end_value = if let Some(params) = params {
        params.end_value
    } else {
        ctx.accounts.destination_token_account.amount
    };
    Ok(end_value)
}
