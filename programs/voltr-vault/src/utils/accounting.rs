use crate::{
    error::VaultError,
    utils::{to_decimal_bits, Decimal, DecimalExtra, SafeMath},
};
use anchor_lang::prelude::*;
use num_traits::Zero;

use super::constants::{MAX_FEE_BPS, ONE_YEAR_U64};

pub fn calc_init_lp_to_mint(amount: u64, from_decimals: u8, to_decimals: u8) -> Result<u64> {
    // If the pool is empty, return amount normalised in LP decimals
    Ok(u64::try_from(
        u128::from(amount)
            .safe_mul((10_u128).pow(u32::from(to_decimals)))?
            .safe_div((10_u128).pow(u32::from(from_decimals)))?,
    )?)
}

pub fn calc_deposit_lp_to_mint(
    amount: u64,
    total_lp_supply_pre_deposit: u64,
    total_asset_pre_deposit: u64,
    issuance_fee_bps: u16,
) -> Result<u64> {
    // maintain ratio of total_lp_supply / total_asset
    // take into account of issuance fee
    // x = lp_to_mint
    // y = total_lp_supply_pre_deposit
    // z = total_asset_post_deposit
    // a = amount
    // i = issuance_fee_bps
    // x / (y + x) * z = a * (10_000 - i) / 10_000
    // x = (a* (10_000 - i) * y ) / (10_000 * z - a * (10_000 - i))
    // Avoid division by zero. Assuming total_asset > 0 if total_lp_supply > 0.

    let total_asset_post_deposit = total_asset_pre_deposit.safe_add(amount)?;
    let numerator = u128::from(amount)
        .safe_mul(u128::from(total_lp_supply_pre_deposit))?
        .safe_mul(u128::from(MAX_FEE_BPS.safe_sub(issuance_fee_bps)?))?;

    let denominator = u128::from(total_asset_post_deposit)
        .safe_mul(u128::from(MAX_FEE_BPS))?
        .safe_sub(
            u128::from(amount).safe_mul(u128::from(MAX_FEE_BPS.safe_sub(issuance_fee_bps)?))?,
        )?;

    if denominator.is_zero() {
        return Err(VaultError::DivisionByZero.into());
    }

    let lp_to_mint = u64::try_from(numerator.safe_div(denominator)?)?;

    Ok(lp_to_mint)
}

pub fn calc_fee_lp_to_mint(
    fee_amount: u64,
    total_lp_supply_pre_fee: u64,
    total_asset_post_fee: u64,
) -> Result<u64> {
    // We want to receive exactly fee_amount (in asset terms) worth of LP tokens
    // lp_to_mint = (fee_amount * total_lp_supply) / (total_assets - fee_amount)
    // Avoid division by zero. Assuming total_asset > 0 if total_lp_supply > 0.
    let lp_numerator = u128::from(fee_amount).safe_mul(u128::from(total_lp_supply_pre_fee))?;

    let denominator = u128::from(total_asset_post_fee.safe_sub(fee_amount)?);

    if denominator.is_zero() {
        return Err(VaultError::DivisionByZero.into());
    }

    let lp_to_mint = u64::try_from(lp_numerator.safe_div_ceil(denominator)?)?;

    Ok(lp_to_mint)
}

pub fn calc_withdraw_lp_to_burn(
    redeem_amount_decimal_bits: u128,
    total_lp_supply_pre_withdraw: u64,
    total_asset_pre_withdraw: u64,
    redemption_fee_bps: u16,
) -> Result<u64> {
    // FORMULA:
    // lp_to_burn_pre_fee = redeem_amount * (total_lp_supply_pre_withdraw / total_asset_pre_withdraw)
    // lp_to_burn_post_fee = lp_to_burn_pre_fee  * (10000 / (10000 - redemption_fee_bps))

    // redeem_amount
    let redeem_amount_decimal = Decimal::from_bits(redeem_amount_decimal_bits);

    // lp_to_burn_pre_fee = redeem_amount * (total_lp_supply_pre_withdraw / total_asset_pre_withdraw)
    let lp_to_burn_pre_fee_decimal = redeem_amount_decimal
        .full_mul_int_ratio(total_lp_supply_pre_withdraw, total_asset_pre_withdraw);

    let const_bps_sub_fee_bps = MAX_FEE_BPS.saturating_sub(redemption_fee_bps);

    if const_bps_sub_fee_bps.is_zero() {
        return Err(VaultError::DivisionByZero.into());
    }

    // lp_to_burn_post_fee = lp_to_burn_pre_fee  * (10000 / (10000 - redemption_fee_bps))
    let lp_to_burn_post_fee_decimal =
        lp_to_burn_pre_fee_decimal.full_mul_int_ratio(MAX_FEE_BPS, const_bps_sub_fee_bps);

    Ok(lp_to_burn_post_fee_decimal.to_ceil())
}

pub fn get_capped_withdraw_asset_to_redeem(
    initial_amount_asset_to_withdraw_decimal_bits: u128,
    amount_lp_to_burn: u64,
    total_lp_supply_pre_withdraw: u64,
    total_asset_pre_withdraw: u64,
    redemption_fee_bps: u16,
) -> Result<u64> {
    // Get the capped amount of asset to withdraw
    // If the latest amount of asset to withdraw is greater than the initial amount of asset to withdraw,
    // then return the initial amount of asset to withdraw
    // Otherwise, return the latest amount of asset to withdraw

    let latest_amount_asset_to_withdraw_decimal_bits = calc_withdraw_asset_to_redeem(
        amount_lp_to_burn,
        total_lp_supply_pre_withdraw,
        total_asset_pre_withdraw,
        redemption_fee_bps,
    )?;

    let amount_asset_to_withdraw_decimal =
        Decimal::from_bits(latest_amount_asset_to_withdraw_decimal_bits).min(Decimal::from_bits(
            initial_amount_asset_to_withdraw_decimal_bits,
        ));

    Ok(amount_asset_to_withdraw_decimal.to_floor())
}

pub fn calc_withdraw_asset_to_redeem(
    amount_lp_to_burn: u64,
    total_lp_supply_pre_withdraw: u64,
    total_asset_pre_withdraw: u64,
    redemption_fee_bps: u16,
) -> Result<u128> {
    // FORMULA:
    // asset_to_redeem_pre_fee = amount_lp_to_burn * (total_asset_pre_withdraw / total_lp_supply_pre_withdraw)
    // asset_to_redeem_post_fee = asset_to_redeem_pre_fee * (10000 - redemption_fee_bps) / 10000

    let amount_lp_to_burn_decimal = Decimal::from_num(amount_lp_to_burn);

    // asset_to_redeeem_pre_fee = amount_lp_to_burn * (total_asset_pre_withdraw / total_lp_supply_pre_withdraw)
    let asset_to_redeeem_pre_fee_decimal = amount_lp_to_burn_decimal
        .full_mul_int_ratio(total_asset_pre_withdraw, total_lp_supply_pre_withdraw);

    // asset_to_redeem_post_fee = asset_to_redeem_pre_fee * (10000 - redemption_fee_bps) / 10000
    let asset_to_redeem_post_fee_decimal = asset_to_redeeem_pre_fee_decimal
        .full_mul_int_ratio(MAX_FEE_BPS.saturating_sub(redemption_fee_bps), MAX_FEE_BPS);

    Ok(asset_to_redeem_post_fee_decimal.to_decimal_bits())
}

pub fn calc_withdraw_pnl(
    pre_invoke_amount: u64,
    post_invoke_amount: u64,
    withdrawn_amount: u64,
) -> Result<(bool, u64)> {
    let (is_pos_expected, expected_amount) = if pre_invoke_amount >= withdrawn_amount {
        (true, pre_invoke_amount.safe_sub(withdrawn_amount)?)
    } else {
        (false, withdrawn_amount.safe_sub(pre_invoke_amount)?)
    };

    // Compare with actual new amount to determine if it's a profit or loss
    let (is_profit, pnl_amount) = if !is_pos_expected {
        (true, post_invoke_amount.safe_add(expected_amount)?)
    } else if post_invoke_amount >= expected_amount {
        (true, post_invoke_amount.safe_sub(expected_amount)?)
    } else {
        (false, expected_amount.safe_sub(post_invoke_amount)?)
    };

    Ok((is_profit, pnl_amount))
}

pub fn calc_deposit_pnl(
    pre_invoke_amount: u64,
    post_invoke_amount: u64,
    deposited_amount: u64,
) -> Result<(bool, u64)> {
    let expected_amount = pre_invoke_amount.safe_add(deposited_amount)?;

    // Compare with actual new amount to determine if it's a profit or loss
    if post_invoke_amount >= expected_amount {
        Ok((true, post_invoke_amount.safe_sub(expected_amount)?))
    } else {
        Ok((false, expected_amount.safe_sub(post_invoke_amount)?))
    }
}

pub fn get_amount_lp_to_burn_and_asset_to_withdraw(
    withdraw_amount: u64,
    user_lp_amount: u64,
    total_lp_supply_incl_fees: u64,
    total_unlocked_asset: u64,
    redemption_fee_bps: u16,
    is_amount_in_lp: bool,
    is_withdraw_all: bool,
) -> Result<(u64, u128)> {
    if is_withdraw_all {
        // Calculate how many asset tokens to give back
        let amount_asset_to_withdraw = calc_withdraw_asset_to_redeem(
            user_lp_amount,
            total_lp_supply_incl_fees,
            total_unlocked_asset,
            redemption_fee_bps,
        )?;

        Ok((user_lp_amount, amount_asset_to_withdraw))
    } else if is_amount_in_lp {
        // Calculate how many asset tokens to give back
        let amount_asset_to_withdraw = calc_withdraw_asset_to_redeem(
            withdraw_amount,
            total_lp_supply_incl_fees,
            total_unlocked_asset,
            redemption_fee_bps,
        )?;

        Ok((withdraw_amount, amount_asset_to_withdraw))
    } else {
        // Calculate how many LP tokens to burn
        let amount_lp_to_burn = calc_withdraw_lp_to_burn(
            to_decimal_bits(withdraw_amount),
            total_lp_supply_incl_fees,
            total_unlocked_asset,
            redemption_fee_bps,
        )?;

        Ok((amount_lp_to_burn, to_decimal_bits(withdraw_amount)))
    }
}

pub fn calc_cancel_withdraw_amount_lp_to_refund_and_burn(
    initial_amount_lp_to_refund: u64,
    initial_amount_asset_worth_decimal_bits: u128,
    total_lp_supply_incl_fees: u64,
    total_asset: u64,
    redemption_fee_bps: u16,
) -> Result<(u64, u64)> {
    // clp = current lp to refund
    // ilp = initial lp to refund
    // ia = initial asset worth
    // ta = total asset
    // tlp = total lp supply with fees
    // r = redemption fee bps
    // user_share = clp / (tlp - ilp + clp)
    // formula = ta * clp / (tlp - ilp + clp) * (10000 - r) / 10000 = sa
    // clp = ia * (tlp - ilp) * (10000 - r) / (ta * 10000 - ia * (10000 - r))
    let initial_amount_asset_worth_decimal =
        Decimal::from_bits(initial_amount_asset_worth_decimal_bits);

    let initial_amount_asset_worth_u64: u64 = initial_amount_asset_worth_decimal.to_floor();
    if total_lp_supply_incl_fees == initial_amount_lp_to_refund
        || total_asset <= initial_amount_asset_worth_u64
    {
        return Ok((initial_amount_lp_to_refund, 0));
    }

    // (tlp-ilp) * (10000 - r)
    let tlp_sub_ilp_mul_const_bps_sub_fee_bps_decimal = u128::from(total_lp_supply_incl_fees)
        .safe_sub(u128::from(initial_amount_lp_to_refund))?
        .safe_mul(u128::from(MAX_FEE_BPS.saturating_sub(redemption_fee_bps)))?;

    // ta * 10000 - ia * (10000 - r)
    let ta_mul_const_bps_sub_ia_mul_const_bps_sub_fee_bps_decimal = u128::from(total_asset)
        .safe_mul(u128::from(MAX_FEE_BPS))?
        .safe_sub(
            u128::from(initial_amount_asset_worth_u64)
                .safe_mul(u128::from(MAX_FEE_BPS.saturating_sub(redemption_fee_bps)))?,
        )?;

    if ta_mul_const_bps_sub_ia_mul_const_bps_sub_fee_bps_decimal.is_zero() {
        return Err(VaultError::DivisionByZero.into());
    }

    // clp = ia * (tlp - ilp) * (10000 - r) / (ta * 10000 - ia * (10000 - r))
    let current_amount_lp_to_refund_decimal = initial_amount_asset_worth_decimal
        .full_mul_int_ratio(
            tlp_sub_ilp_mul_const_bps_sub_fee_bps_decimal,
            ta_mul_const_bps_sub_ia_mul_const_bps_sub_fee_bps_decimal,
        );

    let current_amount_lp_to_refund: u64 = current_amount_lp_to_refund_decimal.to_floor();

    // we only reduce lp to refund when vault asset per lp increased, we do not increase
    let amount_lp_to_refund: u64 = current_amount_lp_to_refund.min(initial_amount_lp_to_refund);

    let amount_lp_to_burn = initial_amount_lp_to_refund.safe_sub(amount_lp_to_refund)?;

    Ok((amount_lp_to_refund, amount_lp_to_burn))
}

pub fn calc_management_fee_amount_in_asset(
    time_elapsed: u64,
    total_asset_value: u64,
    management_fee_bps: u16,
) -> Result<u64> {
    let fee_amount = u64::try_from(
        u128::from(total_asset_value)
            .safe_mul(u128::from(time_elapsed))?
            .safe_mul(u128::from(management_fee_bps))?
            .safe_div_ceil(u128::from(u64::from(MAX_FEE_BPS) * ONE_YEAR_U64))?,
    )?;

    Ok(fee_amount)
}

pub fn calc_performance_fee_amount_in_asset(
    eligible_profit: u64,
    performance_fee_bps: u16,
) -> Result<u64> {
    Ok(u64::try_from(
        u128::from(eligible_profit)
            .safe_mul(u128::from(performance_fee_bps))?
            .safe_div_ceil(u128::from(MAX_FEE_BPS))?,
    )?)
}

pub fn calc_fee_split(
    lp_fee_amount: u64,
    protocol_fee_bps: u16,
    admin_fee_bps: u16,
    manager_fee_bps: u16,
) -> Result<(u64, u64, u64)> {
    let total_fee_bps = protocol_fee_bps
        .safe_add(admin_fee_bps)?
        .safe_add(manager_fee_bps)?;

    let protocol_fee_amount = u64::try_from(
        u128::from(lp_fee_amount)
            .safe_mul(u128::from(protocol_fee_bps))?
            .safe_div_ceil(u128::from(total_fee_bps))?,
    )?;

    let remaining_lp_fee_amount = lp_fee_amount.safe_sub(protocol_fee_amount)?;

    let admin_manager_total_bps = admin_fee_bps.safe_add(manager_fee_bps)?;

    let (admin_amount, manager_amount) = if admin_manager_total_bps == 0 {
        (0u64, 0u64)
    } else {
        let admin = u64::try_from(
            u128::from(remaining_lp_fee_amount)
                .safe_mul(u128::from(admin_fee_bps))?
                .safe_div_ceil(u128::from(admin_manager_total_bps))?,
        )?;
        let manager = remaining_lp_fee_amount.safe_sub(admin)?;
        (admin, manager)
    };

    Ok((admin_amount, manager_amount, protocol_fee_amount))
}

pub fn calc_fee_lp_to_burn(
    fee_amount: u64,
    total_asset: u64,
    current_total_lp_supply: u64,
) -> Result<u64> {
    // (total_asset / total_lp_1) - (total_asset / total_lp_0) = fee / total_lp_1
    // lp_to_burn = total_lp_0 - total_lp_1 = fee * total_lp_0 / total_asset
    if total_asset.is_zero() {
        return Err(VaultError::DivisionByZero.into());
    }

    let fee_amount_decimal = Decimal::from_num(fee_amount);
    let total_asset_decimal = Decimal::from_num(total_asset);
    let current_total_lp_supply_decimal = Decimal::from_num(current_total_lp_supply);

    let lp_per_asset_decimal = current_total_lp_supply_decimal.safe_div(total_asset_decimal)?;
    let lp_to_burn_decimal = fee_amount_decimal.safe_mul(lp_per_asset_decimal)?;

    Ok(lp_to_burn_decimal.to_ceil())
}

pub fn calc_eligible_profit_with_decimal_bits(
    high_water_mark_decimal_bits: u128,
    current_asset_per_lp_decimal_bits: u128,
    current_total_lp_supply: u64,
) -> Result<u64> {
    let high_water_mark_decimal = Decimal::from_bits(high_water_mark_decimal_bits);
    let current_asset_per_lp_decimal = Decimal::from_bits(current_asset_per_lp_decimal_bits);

    if high_water_mark_decimal.ge(&current_asset_per_lp_decimal) {
        Ok(0)
    } else {
        let profit_per_lp_decimal =
            current_asset_per_lp_decimal.safe_sub(high_water_mark_decimal)?;

        let eligible_profit_decimal =
            profit_per_lp_decimal.safe_mul(Decimal::from_num(current_total_lp_supply))?;

        let eligible_profit = eligible_profit_decimal.to_ceil();

        Ok(eligible_profit)
    }
}
