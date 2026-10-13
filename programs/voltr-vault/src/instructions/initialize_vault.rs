use anchor_lang::{
    prelude::*,
    solana_program::{program_pack::Pack, system_instruction},
};
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, AssociatedToken},
    token::{spl_token, InitializeMint, Token},
    token_interface::{Mint, TokenInterface},
};

use crate::{
    error::VaultError,
    state::{
        FeeUpdate, InitializeVaultEvent, Vault, VaultAsset, VaultConfiguration,
        VaultInitializationInput,
    },
    utils::{
        constants::{
            PROTOCOL_SEED, VAULT_ASSET_IDLE_AUTH_SEED, VAULT_LP_MINT_AUTH_SEED, VAULT_LP_MINT_SEED,
        },
        Decimal,
    },
    FeeConfiguration, FeeState, HighWaterMark, OperationalState, Protocol, VaultLp,
};

pub fn handler_initialize_vault(
    ctx: Context<InitializeVault>,
    config: VaultInitializationInput,
    name: String,
    description: String,
) -> Result<()> {
    ctx.accounts
        .protocol
        .load()?
        .check_operation_state(OperationalState::CREATE_VAULT)?;
    config.validate_fees()?;

    ctx.accounts
        .create_and_init_token_mint(ctx.bumps.vault_lp_mint)?;

    // Initialize the vault account
    let vault = &mut ctx.accounts.vault.load_init()?;

    require!(name.as_bytes().len() <= 32, VaultError::InvalidInput);
    let mut name_data = [0u8; 32];
    name_data[..name.as_bytes().len()].copy_from_slice(name.as_bytes());

    require!(description.as_bytes().len() <= 64, VaultError::InvalidInput);
    let mut description_data = [0u8; 64];
    description_data[..description.as_bytes().len()].copy_from_slice(description.as_bytes());

    vault.name = name_data;
    vault.description = description_data;
    vault.asset = VaultAsset {
        mint: ctx.accounts.vault_asset_mint.key(),
        idle_ata: ctx.accounts.vault_asset_idle_ata.key(),
        total_value: 0,
        idle_ata_auth_bump: ctx.bumps.vault_asset_idle_auth,
        reserved: [0u8; 95],
    };
    vault.lp = VaultLp {
        mint: ctx.accounts.vault_lp_mint.key(),
        mint_bump: ctx.bumps.vault_lp_mint,
        mint_auth_bump: ctx.bumps.vault_lp_mint_auth,
        reserved: [0u8; 62],
    };
    vault.manager = ctx.accounts.manager.key();
    vault.admin = ctx.accounts.admin.key();
    vault.vault_configuration = VaultConfiguration::from(config);
    vault.fee_configuration = FeeConfiguration::from(config);
    vault.fee_state = FeeState {
        accumulated_lp_manager_fees: 0,
        accumulated_lp_admin_fees: 0,
        accumulated_lp_protocol_fees: 0,
        reserved: [0u8; 16],
    };
    vault.dead_weight = 0;
    vault.high_water_mark = HighWaterMark {
        highest_asset_per_lp_decimal_bits: Decimal::ZERO.to_bits(),
        last_updated_ts: 0,
        reserved: [0u8; 8],
    };
    vault.fee_update = FeeUpdate {
        last_management_fee_update_ts: u64::try_from(ctx.accounts.clock.unix_timestamp)?,
        last_performance_fee_update_ts: 0,
    };
    vault.version = 4;

    ctx.accounts.create_asset_idle_ata()?;

    vault.last_updated_ts = u64::try_from(ctx.accounts.clock.unix_timestamp)?;

    emit!(InitializeVaultEvent {
        payer: ctx.accounts.payer.key(),
        vault: ctx.accounts.vault.key(),
        vault_name: name,
        vault_description: description,
        vault_asset_mint: ctx.accounts.vault_asset_mint.key(),
        vault_asset_idle_ata: ctx.accounts.vault_asset_idle_ata.key(),
        vault_lp_mint: ctx.accounts.vault_lp_mint.key(),
        vault_manager: ctx.accounts.manager.key(),
        vault_admin: ctx.accounts.admin.key(),
        vault_config_max_cap: config.max_cap,
        vault_config_start_at_ts: config.start_at_ts,
        vault_config_locked_profit_degradation_duration: config.locked_profit_degradation_duration,
        vault_config_withdrawal_waiting_period: config.withdrawal_waiting_period,
        vault_config_manager_performance_fee: config.manager_performance_fee,
        vault_config_admin_performance_fee: config.admin_performance_fee,
        vault_config_manager_management_fee: config.manager_management_fee,
        vault_config_admin_management_fee: config.admin_management_fee,
        vault_config_redemption_fee: config.redemption_fee,
        vault_config_issuance_fee: config.issuance_fee,
        initialized_ts: vault.last_updated_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK:
    #[account()]
    pub manager: AccountInfo<'info>,

    /// CHECK:
    #[account()]
    pub admin: AccountInfo<'info>,

    #[account(seeds = [PROTOCOL_SEED], bump = protocol.load()?.bump)]
    pub protocol: AccountLoader<'info, Protocol>,

    #[account(init, payer = payer, space = std::mem::size_of::<Vault>() + 8)]
    pub vault: AccountLoader<'info, Vault>,

    /// CHECK:
    #[account(
        mut,
        seeds = [VAULT_LP_MINT_SEED, vault.key().as_ref()],
        bump
    )]
    pub vault_lp_mint: AccountInfo<'info>,

    #[account(mint::token_program = asset_token_program)]
    pub vault_asset_mint: InterfaceAccount<'info, Mint>,

    /// CHECK:
    #[account(
        mut,
        constraint =
            vault_asset_idle_ata.key() ==
            get_associated_token_address_with_program_id(
                &vault_asset_idle_auth.key(),
                &vault_asset_mint.key(),
                &asset_token_program.key()
            )
            @ VaultError::InvalidTokenAccount
    )]
    pub vault_asset_idle_ata: AccountInfo<'info>,

    /// CHECK:
    #[account(seeds = [VAULT_LP_MINT_AUTH_SEED, vault.key().as_ref()], bump)]
    pub vault_lp_mint_auth: AccountInfo<'info>,

    /// CHECK:
    #[account(seeds = [VAULT_ASSET_IDLE_AUTH_SEED, vault.key().as_ref()], bump)]
    pub vault_asset_idle_auth: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub asset_token_program: Interface<'info, TokenInterface>,
    pub lp_token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeVault<'info> {
    fn create_and_init_token_mint(&self, lp_mint_bump: u8) -> Result<()> {
        let rent_exemption = self.rent.minimum_balance(spl_token::state::Mint::LEN);
        let create_mint_ix = system_instruction::create_account(
            &self.payer.key(),
            &self.vault_lp_mint.key(),
            rent_exemption,
            u64::try_from(spl_token::state::Mint::LEN)?,
            &self.lp_token_program.key(),
        );

        let reserve_lp_mint_seeds = &[
            VAULT_LP_MINT_SEED,
            &self.vault.key().to_bytes(),
            &[lp_mint_bump],
        ];

        solana_invoke::invoke_signed(
            &create_mint_ix,
            &[
                self.payer.to_account_info(),
                self.vault_lp_mint.clone(),
                self.system_program.to_account_info(),
            ],
            &[reserve_lp_mint_seeds],
        )?;

        anchor_spl::token::initialize_mint(
            CpiContext::new_with_signer(
                self.lp_token_program.to_account_info(),
                InitializeMint {
                    mint: self.vault_lp_mint.clone(),
                    rent: self.rent.to_account_info(),
                },
                &[reserve_lp_mint_seeds],
            ),
            9,
            &self.vault_lp_mint_auth.key(),
            None,
        )?;

        Ok(())
    }

    fn create_asset_idle_ata(&self) -> Result<()> {
        anchor_spl::associated_token::create(CpiContext::new(
            self.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: self.payer.to_account_info(),
                associated_token: self.vault_asset_idle_ata.to_account_info(),
                authority: self.vault_asset_idle_auth.to_account_info(),
                mint: self.vault_asset_mint.to_account_info(),
                system_program: self.system_program.to_account_info(),
                token_program: self.asset_token_program.to_account_info(),
            },
        ))
    }
}
