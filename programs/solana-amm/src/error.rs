use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Math operation overflowed")]
    MathOverflow,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Insufficient liquidity in pool")]
    InsufficientLiquidity,
    #[msg("Output amount is less than minimum specified (slippage exceeded)")]
    SlippageExceeded,
    #[msg("Required input amount exceeds maximum specified")]
    MaxInputExceeded,
    #[msg("Pool is currently locked / paused")]
    PoolLocked,
    #[msg("Invalid fee configuration: protocol fee must not exceed total fee")]
    InvalidFee,
    #[msg("Total fee exceeds maximum allowed basis points (1000 bps / 10%)")]
    FeeTooHigh,
    #[msg("Invariant check failed: invariant decreased")]
    InvariantViolated,
    #[msg("Invalid token mints: mint_x and mint_y must be distinct")]
    IdenticalMints,
    #[msg("Invalid treasury account: token account does not match expected mint or owner")]
    InvalidTreasury,
    #[msg("Unauthorized: caller is not the admin authority")]
    Unauthorized,
}
