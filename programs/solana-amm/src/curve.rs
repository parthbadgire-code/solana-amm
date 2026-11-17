use crate::constants::BPS_DENOMINATOR;
use crate::error::AmmError;
use anchor_lang::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapResult {
    /// Amount of output token the user receives
    pub amount_out: u64,
    /// Total fee deducted from the input amount
    pub total_fee: u64,
    /// Protocol fee routed to the treasury
    pub protocol_fee: u64,
    /// LP fee retained in the pool
    pub lp_fee: u64,
    /// Net input amount added to the pool for swap curve calculation
    pub effective_amount_in: u64,
}

/// Computes integer square root using the Babylonian / Newton-Raphson method
pub fn sqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x0 = n / 2;
    if x0 == 0 {
        return 1;
    }
    let mut x1 = (x0 + n / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + n / x0) / 2;
    }
    x0
}

/// Calculates initial LP token amount to mint on first deposit
/// LP = floor(sqrt(x * y))
pub fn calculate_initial_lp(amount_x: u64, amount_y: u64) -> Result<u64> {
    require!(amount_x > 0 && amount_y > 0, AmmError::ZeroAmount);

    let product = (amount_x as u128)
        .checked_mul(amount_y as u128)
        .ok_or(AmmError::MathOverflow)?;

    let lp = sqrt_u128(product);
    require!(lp > 0, AmmError::InsufficientLiquidity);

    let lp_u64 = u64::try_from(lp).map_err(|_| AmmError::MathOverflow)?;
    Ok(lp_u64)
}

/// Calculates required deposit amounts (x, y) to mint an exact amount of LP tokens
/// dx = ceil((amount_lp * reserve_x) / total_lp)
/// dy = ceil((amount_lp * reserve_y) / total_lp)
/// Uses ceiling division so the pool is protected from rounding dilution
pub fn calculate_deposit_amounts_from_lp(
    amount_lp: u64,
    reserve_x: u64,
    reserve_y: u64,
    total_lp: u64,
) -> Result<(u64, u64)> {
    require!(amount_lp > 0, AmmError::ZeroAmount);
    require!(reserve_x > 0 && reserve_y > 0 && total_lp > 0, AmmError::InsufficientLiquidity);

    let amount_lp_u128 = amount_lp as u128;
    let total_lp_u128 = total_lp as u128;

    // dx = (amount_lp * reserve_x + total_lp - 1) / total_lp (ceiling)
    let num_x = amount_lp_u128
        .checked_mul(reserve_x as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_add(total_lp_u128 - 1)
        .ok_or(AmmError::MathOverflow)?;
    let dx = num_x
        .checked_div(total_lp_u128)
        .ok_or(AmmError::MathOverflow)?;

    // dy = (amount_lp * reserve_y + total_lp - 1) / total_lp (ceiling)
    let num_y = amount_lp_u128
        .checked_mul(reserve_y as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_add(total_lp_u128 - 1)
        .ok_or(AmmError::MathOverflow)?;
    let dy = num_y
        .checked_div(total_lp_u128)
        .ok_or(AmmError::MathOverflow)?;

    let dx_u64 = u64::try_from(dx).map_err(|_| AmmError::MathOverflow)?;
    let dy_u64 = u64::try_from(dy).map_err(|_| AmmError::MathOverflow)?;

    Ok((dx_u64, dy_u64))
}

/// Calculates tokens returned when burning LP tokens on withdrawal
/// dx = floor((amount_lp * reserve_x) / total_lp)
/// dy = floor((amount_lp * reserve_y) / total_lp)
/// Uses floor division so liquidity providers cannot withdraw more than their fair share
pub fn calculate_withdraw_amounts(
    amount_lp: u64,
    reserve_x: u64,
    reserve_y: u64,
    total_lp: u64,
) -> Result<(u64, u64)> {
    require!(amount_lp > 0, AmmError::ZeroAmount);
    require!(reserve_x > 0 && reserve_y > 0 && total_lp > 0, AmmError::InsufficientLiquidity);
    require!(amount_lp <= total_lp, AmmError::InsufficientLiquidity);

    let amount_lp_u128 = amount_lp as u128;
    let total_lp_u128 = total_lp as u128;

    let dx = amount_lp_u128
        .checked_mul(reserve_x as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(total_lp_u128)
        .ok_or(AmmError::MathOverflow)?;

    let dy = amount_lp_u128
        .checked_mul(reserve_y as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(total_lp_u128)
        .ok_or(AmmError::MathOverflow)?;

    let dx_u64 = u64::try_from(dx).map_err(|_| AmmError::MathOverflow)?;
    let dy_u64 = u64::try_from(dy).map_err(|_| AmmError::MathOverflow)?;

    Ok((dx_u64, dy_u64))
}

/// Calculates swap output, fees, and verifies constant product invariant k = x * y
/// - total_fee = floor((amount_in * fee_bps) / 10000)
/// - protocol_fee = floor((amount_in * protocol_fee_bps) / 10000)
/// - lp_fee = total_fee - protocol_fee
/// - effective_amount_in = amount_in - total_fee
/// - amount_out = floor((reserve_out * effective_amount_in) / (reserve_in + effective_amount_in))
/// - Invariant check: (reserve_in + amount_in - protocol_fee) * (reserve_out - amount_out) >= reserve_in * reserve_out
pub fn calculate_swap_output(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
    protocol_fee_bps: u16,
) -> Result<SwapResult> {
    require!(amount_in > 0, AmmError::ZeroAmount);
    require!(reserve_in > 0 && reserve_out > 0, AmmError::InsufficientLiquidity);
    require!(protocol_fee_bps <= fee_bps, AmmError::InvalidFee);

    let amount_in_u128 = amount_in as u128;
    let reserve_in_u128 = reserve_in as u128;
    let reserve_out_u128 = reserve_out as u128;

    // Fee breakdown
    let total_fee_u128 = amount_in_u128
        .checked_mul(fee_bps as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(AmmError::MathOverflow)?;

    let protocol_fee_u128 = amount_in_u128
        .checked_mul(protocol_fee_bps as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(AmmError::MathOverflow)?;

    let lp_fee_u128 = total_fee_u128
        .checked_sub(protocol_fee_u128)
        .ok_or(AmmError::MathOverflow)?;

    let effective_in_u128 = amount_in_u128
        .checked_sub(total_fee_u128)
        .ok_or(AmmError::MathOverflow)?;

    require!(effective_in_u128 > 0, AmmError::ZeroAmount);

    // Constant product swap formula: dy = (reserve_out * dx_eff) / (reserve_in + dx_eff)
    let numerator = reserve_out_u128
        .checked_mul(effective_in_u128)
        .ok_or(AmmError::MathOverflow)?;

    let denominator = reserve_in_u128
        .checked_add(effective_in_u128)
        .ok_or(AmmError::MathOverflow)?;

    let amount_out_u128 = numerator
        .checked_div(denominator)
        .ok_or(AmmError::MathOverflow)?;

    require!(amount_out_u128 > 0, AmmError::ZeroAmount);
    require!(amount_out_u128 < reserve_out_u128, AmmError::InsufficientLiquidity);

    // Invariant verification: k_new >= k_old
    // Pool gains (amount_in - protocol_fee) in reserve_in, and loses amount_out from reserve_out
    let new_reserve_in = reserve_in_u128
        .checked_add(amount_in_u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_sub(protocol_fee_u128)
        .ok_or(AmmError::MathOverflow)?;

    let new_reserve_out = reserve_out_u128
        .checked_sub(amount_out_u128)
        .ok_or(AmmError::MathOverflow)?;

    let k_old = reserve_in_u128
        .checked_mul(reserve_out_u128)
        .ok_or(AmmError::MathOverflow)?;

    let k_new = new_reserve_in
        .checked_mul(new_reserve_out)
        .ok_or(AmmError::MathOverflow)?;

    require!(k_new >= k_old, AmmError::InvariantViolated);

    Ok(SwapResult {
        amount_out: u64::try_from(amount_out_u128).map_err(|_| AmmError::MathOverflow)?,
        total_fee: u64::try_from(total_fee_u128).map_err(|_| AmmError::MathOverflow)?,
        protocol_fee: u64::try_from(protocol_fee_u128).map_err(|_| AmmError::MathOverflow)?,
        lp_fee: u64::try_from(lp_fee_u128).map_err(|_| AmmError::MathOverflow)?,
        effective_amount_in: u64::try_from(effective_in_u128).map_err(|_| AmmError::MathOverflow)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(sqrt_u128(0), 0);
        assert_eq!(sqrt_u128(1), 1);
        assert_eq!(sqrt_u128(2), 1);
        assert_eq!(sqrt_u128(3), 1);
        assert_eq!(sqrt_u128(4), 2);
        assert_eq!(sqrt_u128(9), 3);
        assert_eq!(sqrt_u128(15), 3);
        assert_eq!(sqrt_u128(16), 4);
        assert_eq!(sqrt_u128(1_000_000_000_000), 1_000_000);
        assert_eq!(sqrt_u128(100_000_000_000_000_000_000_000_000_000_000), 10_000_000_000_000_000);
    }

    #[test]
    fn test_initial_lp() {
        let x = 1_000_000_000;
        let y = 1_000_000_000;
        let lp = calculate_initial_lp(x, y).unwrap();
        assert_eq!(lp, 1_000_000_000);

        let x2 = 400_000;
        let y2 = 900_000;
        let lp2 = calculate_initial_lp(x2, y2).unwrap();
        assert_eq!(lp2, 600_000);
    }

    #[test]
    fn test_deposit_amounts_from_lp() {
        let reserve_x = 1_000_000;
        let reserve_y = 2_000_000;
        let total_lp = 1_000_000;

        // Desired LP = 500_000 (50% of pool)
        let (dx, dy) = calculate_deposit_amounts_from_lp(500_000, reserve_x, reserve_y, total_lp).unwrap();
        assert_eq!(dx, 500_000);
        assert_eq!(dy, 1_000_000);
    }

    #[test]
    fn test_withdraw_amounts() {
        let reserve_x = 1_000_000;
        let reserve_y = 2_000_000;
        let total_lp = 1_000_000;

        // Withdraw 250_000 LP (25% of pool)
        let (dx, dy) = calculate_withdraw_amounts(250_000, reserve_x, reserve_y, total_lp).unwrap();
        assert_eq!(dx, 250_000);
        assert_eq!(dy, 500_000);
    }

    #[test]
    fn test_swap_output_and_fee_split() {
        let reserve_in = 1_000_000_000;
        let reserve_out = 1_000_000_000;
        let amount_in = 10_000_000; // 1% of pool
        let fee_bps = 30; // 0.3% total fee
        let protocol_fee_bps = 5; // 0.05% protocol fee (treasury)

        let result = calculate_swap_output(amount_in, reserve_in, reserve_out, fee_bps, protocol_fee_bps).unwrap();

        // Total fee = 10_000_000 * 30 / 10000 = 30,000
        assert_eq!(result.total_fee, 30_000);
        // Protocol fee = 10_000_000 * 5 / 10000 = 5,000
        assert_eq!(result.protocol_fee, 5_000);
        // LP fee = 30,000 - 5,000 = 25,000
        assert_eq!(result.lp_fee, 25_000);
        // Effective in = 10_000_000 - 30_000 = 9_970_000
        assert_eq!(result.effective_amount_in, 9_970_000);

        // Expected out = (1_000_000_000 * 9_970_000) / (1_000_000_000 + 9_970_000) = 9_871_580
        assert_eq!(result.amount_out, 9_871_580);
    }

    #[test]
    fn test_invariant_increases_with_lp_fee() {
        let reserve_in = 1_000_000_000u128;
        let reserve_out = 1_000_000_000u128;
        let k_old = reserve_in * reserve_out;

        let res = calculate_swap_output(50_000_000, 1_000_000_000, 1_000_000_000, 30, 5).unwrap();

        let new_in = reserve_in + (50_000_000 - res.protocol_fee) as u128;
        let new_out = reserve_out - res.amount_out as u128;
        let k_new = new_in * new_out;

        assert!(k_new >= k_old);
    }
}
