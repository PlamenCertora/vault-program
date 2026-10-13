use anchor_lang::{
    prelude::{AccountInfo, AccountMeta, Pubkey},
    solana_program, Key, Result,
};

pub fn encode_additional_args(data: &mut Vec<u8>, additional_args: &Option<Vec<u8>>) {
    if let Some(args) = additional_args {
        let additional_args_len_bytes = u32::try_from(args.len()).unwrap().to_le_bytes();
        data.push(1); // 1 indicates presence of additional args
        data.extend_from_slice(&additional_args_len_bytes);
        data.extend(args);
    } else {
        data.push(0); // 0 indicates no additional args
    }
}

pub fn to_account_meta_modify_manager_non_signer_option(
    acc: &AccountInfo<'_>,
    manager_key_option: Option<&Pubkey>,
) -> AccountMeta {
    let acc_is_signer = match manager_key_option {
        Some(manager_key) => {
            if manager_key.eq(&acc.key) {
                false
            } else {
                acc.is_signer
            }
        }
        None => acc.is_signer,
    };
    if acc.is_writable {
        AccountMeta::new(acc.key(), acc_is_signer)
    } else {
        AccountMeta::new_readonly(acc.key(), acc_is_signer)
    }
}

pub fn get_return_u64(program_id: Pubkey) -> Result<u64> {
    solana_program::program::get_return_data()
        .ok_or(anchor_lang::solana_program::program_error::ProgramError::InvalidInstructionData)
        .and_then(|(key, data)| {
            if key != program_id {
                Err(anchor_lang::solana_program::program_error::ProgramError::IncorrectProgramId)
            } else {
                data.try_into().map(u64::from_le_bytes).map_err(|_| {
                    anchor_lang::solana_program::program_error::ProgramError::InvalidInstructionData
                })
            }
        })
        .map_err(Into::into)
}
