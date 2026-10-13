pub const PROTOCOL_SEED: &[u8] = b"protocol";
pub const VAULT_LP_MINT_SEED: &[u8] = b"vault_lp_mint";
pub const VAULT_LP_MINT_AUTH_SEED: &[u8] = b"vault_lp_mint_auth";
pub const VAULT_ASSET_IDLE_AUTH_SEED: &[u8] = b"vault_asset_idle_auth";
pub const ADAPTOR_ADD_RECEIPT_SEED: &[u8] = b"adaptor_add_receipt";
pub const STRATEGY_INIT_RECEIPT_SEED: &[u8] = b"strategy_init_receipt";
pub const VAULT_STRATEGY_AUTH_SEED: &[u8] = b"vault_strategy_auth";
pub const VAULT_STRATEGY_SEED: &[u8] = b"vault_strategy";
pub const DIRECT_WITHDRAW_INIT_RECEIPT_SEED: &[u8] = b"direct_withdraw_init_receipt";
pub const REQUEST_WITHDRAW_VAULT_RECEIPT_SEED: &[u8] = b"request_withdraw_vault_receipt";

// HEX_TO_BYTES(SHA256('global:initialize')[0:16]):
pub const INITIALIZE_ANCHOR_DISCRIMINATOR: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
// HEX_TO_BYTES(SHA256('global:deposit')[0:16]):
pub const DEPOSIT_ANCHOR_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
// HEX_TO_BYTES(SHA256('global:withdraw')[0:16]):
pub const WITHDRAW_ANCHOR_DISCRIMINATOR: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];

pub const ONE_YEAR_U64: u64 = 365 * 24 * 60 * 60;
pub const MAX_FEE_BPS: u16 = 10_000;
pub const DEAD_WEIGHT: u64 = 1_000;
