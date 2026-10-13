use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawParams {
    pub placeholder_vec_len: u32,
    pub end_value: u64,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK:
    #[account(seeds = [b"strategy"], bump)]
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
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
}

pub fn withdraw_handler(
    ctx: Context<Withdraw>,
    amount: u64,
    params: Option<WithdrawParams>,
) -> Result<u64> {
    if amount.gt(&0) {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: ctx.accounts.source_token_account.to_account_info(),
                    mint: ctx.accounts.vault_asset_mint.to_account_info(),
                    to: ctx.accounts.authority_token_account.to_account_info(),
                    authority: ctx.accounts.strategy.to_account_info(),
                },
                &[&[b"strategy", &[ctx.bumps.strategy]]],
            ),
            amount,
            ctx.accounts.vault_asset_mint.decimals,
        )?;
        ctx.accounts.source_token_account.reload()?;
    }
    let end_value = if let Some(params) = params {
        params.end_value
    } else {
        ctx.accounts.source_token_account.amount
    };
    Ok(end_value)
}
