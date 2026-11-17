use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Unique seed allowing multiple pools per token pair
    pub seed: u64,
    /// Admin authority who can update fees, treasury, or trigger emergency pause
    pub authority: Pubkey,
    /// Mint address of Token X
    pub mint_x: Pubkey,
    /// Mint address of Token Y
    pub mint_y: Pubkey,
    /// Total swap fee in basis points (e.g. 30 = 0.30%)
    pub fee: u16,
    /// Protocol fee cut in basis points routed to treasury (e.g. 5 = 0.05%)
    pub protocol_fee: u16,
    /// Treasury recipient wallet / authority
    pub treasury: Pubkey,
    /// Emergency circuit breaker: when true, deposits and swaps are frozen
    pub locked: bool,
    /// Bump seed for Config PDA
    pub config_bump: u8,
    /// Bump seed for LP Mint PDA
    pub lp_bump: u8,
}
