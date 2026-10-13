use anchor_lang::prelude::*;

use crate::{
    state::Vault,
    utils::{
        constants::{VAULT_LP_MINT_AUTH_SEED, VAULT_LP_MINT_SEED},
        CreateMetadataAccountV3Cpi, CreateMetadataAccountV3CpiAccounts,
        CreateMetadataAccountV3InstructionArgs, DataV2, UpdateMetadataAccountV2Cpi,
        UpdateMetadataAccountV2CpiAccounts, UpdateMetadataAccountV2InstructionArgs,
        MPL_TOKEN_METADATA_ID,
    },
};

pub fn handler_create_lp_metadata(
    ctx: Context<CreateLpMetadata>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let data_v2 = DataV2 {
        name: name,
        symbol: symbol,
        uri: uri,
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    // check if metadata account is already created
    if ctx.accounts.metadata_account.data_is_empty() {
        let rent_account_info = ctx.accounts.rent.to_account_info();
        let create_metadata_account_v3_cpi = CreateMetadataAccountV3Cpi::new(
            &ctx.accounts.metadata_program,
            CreateMetadataAccountV3CpiAccounts {
                metadata: &ctx.accounts.metadata_account,
                mint: &ctx.accounts.vault_lp_mint,
                mint_authority: &ctx.accounts.vault_lp_mint_auth,
                payer: &ctx.accounts.payer,
                update_authority: (&ctx.accounts.vault_lp_mint_auth, true),
                system_program: &ctx.accounts.system_program,
                rent: Some(&rent_account_info),
            },
            CreateMetadataAccountV3InstructionArgs {
                data: data_v2,
                is_mutable: true,
                collection_details: None,
            },
        );

        create_metadata_account_v3_cpi.invoke_signed(&[&[
            VAULT_LP_MINT_AUTH_SEED,
            &ctx.accounts.vault.key().to_bytes(),
            &[ctx.accounts.vault.load()?.lp.mint_auth_bump],
        ]])?;
    } else {
        let update_metadata_account_v2_cpi = UpdateMetadataAccountV2Cpi::new(
            &ctx.accounts.metadata_program,
            UpdateMetadataAccountV2CpiAccounts {
                metadata: &ctx.accounts.metadata_account,
                update_authority: &ctx.accounts.vault_lp_mint_auth,
            },
            UpdateMetadataAccountV2InstructionArgs {
                data: Some(data_v2),
                update_authority: None,
                primary_sale_happened: None,
                is_mutable: None,
            },
        );

        update_metadata_account_v2_cpi.invoke_signed(&[&[
            VAULT_LP_MINT_AUTH_SEED,
            &ctx.accounts.vault.key().to_bytes(),
            &[ctx.accounts.vault.load()?.lp.mint_auth_bump],
        ]])?;
    }
    Ok(())
}

#[derive(Accounts)]
pub struct CreateLpMetadata<'info> {
    /// CHECK:
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK:
    pub admin: Signer<'info>,

    #[account(has_one = admin)]
    pub vault: AccountLoader<'info, Vault>,

    /// CHECK:
    #[account(
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_bump
    )]
    pub vault_lp_mint: AccountInfo<'info>,

    /// CHECK:
    #[account(
        seeds = [VAULT_LP_MINT_AUTH_SEED, vault.key().as_ref()],
        bump = vault.load()?.lp.mint_auth_bump
    )]
    pub vault_lp_mint_auth: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub metadata_account: AccountInfo<'info>,

    /// CHECK:
    #[account(address = MPL_TOKEN_METADATA_ID)]
    pub metadata_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}
