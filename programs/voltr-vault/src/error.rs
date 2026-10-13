use anchor_lang::error_code;

#[error_code]
pub enum VaultError {
    #[msg("Invalid amount provided.")]
    InvalidAmount,
    #[msg("Invalid token mint.")]
    InvalidTokenMint,
    #[msg("Invalid token account.")]
    InvalidTokenAccount,
    #[msg("Invalid account input.")]
    InvalidAccountInput,
    #[msg("Math overflow.")]
    MathOverflow,
    #[msg("Fee exceeds total asset value.")]
    FeeExceedsTotalAssetValue,
    #[msg("Max cap exceeded.")]
    MaxCapExceeded,
    #[msg("Vault not active.")]
    VaultNotActive,
    #[msg("Manager not allowed in remaining.")]
    ManagerNotAllowed,
    #[msg("Operation not allowed.")]
    OperationNotAllowed,
    #[msg("Adaptor epoch invalid.")]
    AdaptorEpochInvalid,
    #[msg("Fee configuration invalid.")]
    InvalidFeeConfiguration,
    #[msg("Withdrawal not yet available.")]
    WithdrawalNotYetAvailable,
    #[msg("Invalid input.")]
    InvalidInput,
    #[msg("Division by zero.")]
    DivisionByZero,
}
