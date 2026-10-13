use anchor_lang::{prelude::*, solana_program};

pub const MPL_TOKEN_METADATA_ID: Pubkey = pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

#[derive(AnchorDeserialize, AnchorSerialize)]
struct CreateMetadataAccountV3InstructionData {
    discriminator: u8,
}

impl CreateMetadataAccountV3InstructionData {
    fn new() -> Self {
        Self { discriminator: 33 }
    }
}
/// Accounts.
pub struct CreateMetadataAccountV3 {
    /// Metadata key (pda of ['metadata', program id, mint id])
    pub metadata: solana_program::pubkey::Pubkey,
    /// Mint of token asset
    pub mint: solana_program::pubkey::Pubkey,
    /// Mint authority
    pub mint_authority: solana_program::pubkey::Pubkey,
    /// payer
    pub payer: solana_program::pubkey::Pubkey,
    /// update authority info
    pub update_authority: (solana_program::pubkey::Pubkey, bool),
    /// System program
    pub system_program: solana_program::pubkey::Pubkey,
    /// Rent info
    pub rent: Option<solana_program::pubkey::Pubkey>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct DataV2 {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<bool>>,
    pub collection: Option<bool>,
    pub uses: Option<bool>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct CreateMetadataAccountV3InstructionArgs {
    pub data: DataV2,
    pub is_mutable: bool,
    pub collection_details: Option<bool>,
}

/// `create_metadata_account_v3` CPI accounts.
pub struct CreateMetadataAccountV3CpiAccounts<'a, 'b> {
    /// Metadata key (pda of ['metadata', program id, mint id])
    pub metadata: &'b solana_program::account_info::AccountInfo<'a>,
    /// Mint of token asset
    pub mint: &'b solana_program::account_info::AccountInfo<'a>,
    /// Mint authority
    pub mint_authority: &'b solana_program::account_info::AccountInfo<'a>,
    /// payer
    pub payer: &'b solana_program::account_info::AccountInfo<'a>,
    /// update authority info
    pub update_authority: (&'b solana_program::account_info::AccountInfo<'a>, bool),
    /// System program
    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// Rent info
    pub rent: Option<&'b solana_program::account_info::AccountInfo<'a>>,
}

/// `create_metadata_account_v3` CPI instruction.
pub struct CreateMetadataAccountV3Cpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,
    /// Metadata key (pda of ['metadata', program id, mint id])
    pub metadata: &'b solana_program::account_info::AccountInfo<'a>,
    /// Mint of token asset
    pub mint: &'b solana_program::account_info::AccountInfo<'a>,
    /// Mint authority
    pub mint_authority: &'b solana_program::account_info::AccountInfo<'a>,
    /// payer
    pub payer: &'b solana_program::account_info::AccountInfo<'a>,
    /// update authority info
    pub update_authority: (&'b solana_program::account_info::AccountInfo<'a>, bool),
    /// System program
    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// Rent info
    pub rent: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    /// The arguments for the instruction.
    pub __args: CreateMetadataAccountV3InstructionArgs,
}

impl<'a, 'b> CreateMetadataAccountV3Cpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: CreateMetadataAccountV3CpiAccounts<'a, 'b>,
        args: CreateMetadataAccountV3InstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            metadata: accounts.metadata,
            mint: accounts.mint,
            mint_authority: accounts.mint_authority,
            payer: accounts.payer,
            update_authority: accounts.update_authority,
            system_program: accounts.system_program,
            rent: accounts.rent,
            __args: args,
        }
    }
    #[inline(always)]
    pub fn invoke_signed(
        &self,
        signers_seeds: &[&[&[u8]]],
    ) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(signers_seeds, &[])
    }
    #[allow(clippy::clone_on_copy)]
    #[allow(clippy::vec_init_then_push)]
    pub fn invoke_signed_with_remaining_accounts(
        &self,
        signers_seeds: &[&[&[u8]]],
        remaining_accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> solana_program::entrypoint::ProgramResult {
        let mut accounts = Vec::with_capacity(7 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.metadata.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.mint.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.mint_authority.key,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.payer.key,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.update_authority.0.key,
            self.update_authority.1,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.system_program.key,
            false,
        ));
        if let Some(rent) = self.rent {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                *rent.key, false,
            ));
        }
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = CreateMetadataAccountV3InstructionData::new()
            .try_to_vec()
            .unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: MPL_TOKEN_METADATA_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(7 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.metadata.clone());
        account_infos.push(self.mint.clone());
        account_infos.push(self.mint_authority.clone());
        account_infos.push(self.payer.clone());
        account_infos.push(self.update_authority.0.clone());
        account_infos.push(self.system_program.clone());
        if let Some(rent) = self.rent {
            account_infos.push(rent.clone());
        }
        remaining_accounts
            .iter()
            .for_each(|remaining_account| account_infos.push(remaining_account.0.clone()));

        if signers_seeds.is_empty() {
            solana_invoke::invoke(&instruction, &account_infos)
        } else {
            solana_invoke::invoke_signed(&instruction, &account_infos, signers_seeds)
        }
    }
}

#[derive(AnchorDeserialize, AnchorSerialize)]
struct UpdateMetadataAccountV2InstructionData {
    discriminator: u8,
}

impl UpdateMetadataAccountV2InstructionData {
    fn new() -> Self {
        Self { discriminator: 15 }
    }
}

/// Accounts for the `UpdateMetadataAccountV2` instruction.
pub struct UpdateMetadataAccountV2 {
    /// Metadata account
    pub metadata: solana_program::pubkey::Pubkey,
    /// Update authority key
    pub update_authority: solana_program::pubkey::Pubkey,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateMetadataAccountV2InstructionArgs {
    pub data: Option<DataV2>,
    pub update_authority: Option<Pubkey>,
    pub primary_sale_happened: Option<bool>,
    pub is_mutable: Option<bool>,
}

/// `update_metadata_account_v2` CPI accounts.
pub struct UpdateMetadataAccountV2CpiAccounts<'a, 'b> {
    /// Metadata account
    pub metadata: &'b solana_program::account_info::AccountInfo<'a>,
    /// Update authority key
    pub update_authority: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `update_metadata_account_v2` CPI instruction.
pub struct UpdateMetadataAccountV2Cpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,
    /// Metadata account
    pub metadata: &'b solana_program::account_info::AccountInfo<'a>,
    /// Update authority key
    pub update_authority: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: UpdateMetadataAccountV2InstructionArgs,
}

impl<'a, 'b> UpdateMetadataAccountV2Cpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: UpdateMetadataAccountV2CpiAccounts<'a, 'b>,
        args: UpdateMetadataAccountV2InstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            metadata: accounts.metadata,
            update_authority: accounts.update_authority,
            __args: args,
        }
    }
    #[inline(always)]
    pub fn invoke_signed(
        &self,
        signers_seeds: &[&[&[u8]]],
    ) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(signers_seeds, &[])
    }
    #[allow(clippy::clone_on_copy)]
    #[allow(clippy::vec_init_then_push)]
    pub fn invoke_signed_with_remaining_accounts(
        &self,
        signers_seeds: &[&[&[u8]]],
        remaining_accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> solana_program::entrypoint::ProgramResult {
        let mut accounts = Vec::with_capacity(2 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.metadata.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.update_authority.key,
            true,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = UpdateMetadataAccountV2InstructionData::new()
            .try_to_vec()
            .unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: MPL_TOKEN_METADATA_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(2 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.metadata.clone());
        account_infos.push(self.update_authority.clone());
        remaining_accounts
            .iter()
            .for_each(|remaining_account| account_infos.push(remaining_account.0.clone()));

        if signers_seeds.is_empty() {
            solana_program::program::invoke(&instruction, &account_infos)
        } else {
            solana_program::program::invoke_signed(&instruction, &account_infos, signers_seeds)
        }
    }
}
