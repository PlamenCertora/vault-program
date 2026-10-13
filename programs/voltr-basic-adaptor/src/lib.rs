use anchor_lang::prelude::*;
use instructions::*;

pub mod instructions;
declare_id!("92VYotqKr8xZBNZsVhipMeaUmkE5UhPjm3nU99F5ZtJ9");

#[program]
pub mod voltr_basic_adaptor {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize_handler(ctx)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        params: Option<DepositParams>,
    ) -> Result<u64> {
        deposit_handler(ctx, amount, params)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
        params: Option<WithdrawParams>,
    ) -> Result<u64> {
        withdraw_handler(ctx, amount, params)
    }
}
