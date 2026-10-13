use anchor_lang::prelude::*;

use crate::utils::SafeMath;

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct RequestWithdrawVaultReceipt {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub amount_lp_escrowed: u64,
    pub amount_asset_to_withdraw_decimal_bits: u128,
    pub withdrawable_from_ts: u64,
    pub bump: u8,
    pub version: u8,
}

impl RequestWithdrawVaultReceipt {
    pub fn init(
        &mut self,
        vault: Pubkey,
        user: Pubkey,
        amount_lp_escrowed: u64,
        amount_asset_to_withdraw_decimal_bits: u128,
        withdrawal_waiting_period: u64,
        bump: u8,
        current_ts: u64,
    ) -> Result<()> {
        self.vault = vault;
        self.user = user;
        self.amount_lp_escrowed = amount_lp_escrowed;
        self.amount_asset_to_withdraw_decimal_bits = amount_asset_to_withdraw_decimal_bits;
        self.withdrawable_from_ts = current_ts.safe_add(withdrawal_waiting_period)?;
        self.bump = bump;
        self.version = 0;

        Ok(())
    }

    pub fn is_withdrawable(&self, current_ts: u64) -> Result<bool> {
        Ok(current_ts.ge(&self.withdrawable_from_ts))
    }
}

#[cfg(test)]
mod tests {
    use crate::state::RequestWithdrawVaultReceipt;
    use anchor_lang::prelude::*;

    fn create_test_request_withdraw_vault_receipt() -> RequestWithdrawVaultReceipt {
        RequestWithdrawVaultReceipt {
            vault: Pubkey::default(),
            user: Pubkey::default(),
            amount_lp_escrowed: 0,
            amount_asset_to_withdraw_decimal_bits: 0,
            withdrawable_from_ts: 0,
            bump: 0,
            version: 0,
        }
    }

    #[test]
    fn test_request_withdraw_vault_receipt_initialization() {
        let mut request_withdraw_vault_receipt = create_test_request_withdraw_vault_receipt();

        let vault = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let amount_lp_escrowed = 1000;
        let amount_asset_to_withdraw_decimal_bits = 1000;
        let withdrawal_waiting_period = 1000;
        let bump = 255;
        let current_ts = 1725308400;

        request_withdraw_vault_receipt
            .init(
                vault,
                user,
                amount_lp_escrowed,
                amount_asset_to_withdraw_decimal_bits,
                withdrawal_waiting_period,
                bump,
                current_ts,
            )
            .unwrap();

        assert_eq!(request_withdraw_vault_receipt.vault, vault);
        assert_eq!(request_withdraw_vault_receipt.user, user);
        assert_eq!(
            request_withdraw_vault_receipt.amount_lp_escrowed,
            amount_lp_escrowed
        );
        assert_eq!(
            request_withdraw_vault_receipt.amount_asset_to_withdraw_decimal_bits,
            amount_asset_to_withdraw_decimal_bits
        );

        let expected_withdrawable_from_ts = current_ts + withdrawal_waiting_period;

        assert_eq!(
            request_withdraw_vault_receipt.withdrawable_from_ts,
            expected_withdrawable_from_ts
        );
        assert_eq!(request_withdraw_vault_receipt.bump, bump);
        assert_eq!(request_withdraw_vault_receipt.version, 0);
    }

    #[test]
    fn test_request_withdraw_vault_receipt_is_withdrawable() {
        let mut request_withdraw_vault_receipt = create_test_request_withdraw_vault_receipt();

        let withdrawal_waiting_period = 1000;
        let requested_ts = 1725308400;

        request_withdraw_vault_receipt
            .init(
                Pubkey::default(),
                Pubkey::default(),
                1000,
                1000,
                withdrawal_waiting_period,
                255,
                requested_ts,
            )
            .unwrap();

        let current_ts = requested_ts + withdrawal_waiting_period;

        assert_eq!(
            request_withdraw_vault_receipt
                .is_withdrawable(current_ts)
                .unwrap(),
            true
        );
    }

    #[test]
    fn test_request_withdraw_vault_receipt_is_not_withdrawable() {
        let mut request_withdraw_vault_receipt = create_test_request_withdraw_vault_receipt();

        let withdrawal_waiting_period = 1000;
        let requested_ts = 1725308400;

        request_withdraw_vault_receipt
            .init(
                Pubkey::default(),
                Pubkey::default(),
                1000,
                1000,
                withdrawal_waiting_period,
                255,
                requested_ts,
            )
            .unwrap();

        let current_ts = requested_ts + withdrawal_waiting_period - 1;

        assert_eq!(
            request_withdraw_vault_receipt
                .is_withdrawable(current_ts)
                .unwrap(),
            false
        );
    }
}
