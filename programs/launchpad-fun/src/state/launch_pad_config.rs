use anchor_lang::prelude::*;

use anchor_lang::{account, prelude::Pubkey, InitSpace};

use crate::{
    LaunchPadConfigInitialized, LaunchPadErrorCode, LaunchPadPaused, LaunchPadUnpaused,
    ProtocolStatus, DISC_LAUNCH_PAD_CONFIG_ACCOUNT, MAX_PROTOCOL_FEE, MIN_ASSET_RATE,
    MIN_CREATOR_SELL_DELAY, MIN_GRADUATE_THRESHOLD, MIN_PROTOCOL_FEE,
};

#[derive(Default, Debug, InitSpace)]
#[account(discriminator = DISC_LAUNCH_PAD_CONFIG_ACCOUNT)]
pub struct LaunchPadConfig {
    // The authority that can update the launch pad config
    pub authority: Pubkey,
    // The rate of asset per token
    pub asset_rate: u64,
    // The delay in seconds before a creator can sell their tokens
    pub creator_sell_delay: u64,
    // The threshold amount for a token to graduate
    pub graduate_threshold: u64,
    // The protocol buy fee in basis points (10_000 = 1% | 100 = 0.01%) charged on trades
    pub protocol_buy_fee: u32,
    // The protocol sell fee in basis points (10_000 = 1% | 100 = 0.01%) charged on trades
    pub protocol_sell_fee: u32,
    // The current status of the protocol
    pub status: ProtocolStatus,
    // The bump seed for the PDA
    pub bump: u8,
    // The vault bump seed for the PDA
    pub vault_bump: u8,
}

impl LaunchPadConfig {
    pub const SEED: &'static [u8] = b"launch_pad_config:";
    pub const VAULT_SEED: &'static [u8] = b"vault:";

    pub fn initialize(
        &mut self,
        authority: Pubkey,
        asset_rate: u64,
        creator_sell_delay: u64,
        graduate_threshold: u64,
        protocol_buy_fee: u32,
        protocol_sell_fee: u32,
        bump: u8,
        vault_bump: u8,
    ) -> Result<()> {
        require!(
            self.status == ProtocolStatus::Unknown,
            LaunchPadErrorCode::ProtocolConfigInitialized
        );
        require!(
            authority != Pubkey::default(),
            LaunchPadErrorCode::InvalidAuthority
        );
        let time = Clock::get()?.unix_timestamp + MIN_CREATOR_SELL_DELAY as i64;
        require!(
            creator_sell_delay > time as u64,
            LaunchPadErrorCode::CreatorSellDelayNotMet
        );
        require!(
            asset_rate > MIN_ASSET_RATE,
            LaunchPadErrorCode::AssetRateMustBeGreaterThanZero
        );
        require!(
            graduate_threshold > MIN_GRADUATE_THRESHOLD,
            LaunchPadErrorCode::GraduateThresholdNotMet
        );
        require!(
            protocol_buy_fee <= MAX_PROTOCOL_FEE,
            LaunchPadErrorCode::ProtocolFeeExceedsMaximum
        );
        require!(
            protocol_buy_fee >= MIN_PROTOCOL_FEE,
            LaunchPadErrorCode::ProtocolFeeMinimumNotMet
        );
        require!(
            protocol_sell_fee <= MAX_PROTOCOL_FEE,
            LaunchPadErrorCode::ProtocolFeeExceedsMaximum
        );
        require!(
            protocol_sell_fee >= MIN_PROTOCOL_FEE,
            LaunchPadErrorCode::ProtocolFeeMinimumNotMet
        );
        self.authority = authority;
        self.asset_rate = asset_rate;
        self.creator_sell_delay = creator_sell_delay;
        self.graduate_threshold = graduate_threshold;
        self.protocol_buy_fee = protocol_buy_fee;
        self.protocol_sell_fee = protocol_sell_fee;
        self.status = ProtocolStatus::Active;
        self.bump = bump;
        self.vault_bump = vault_bump;

        emit!(LaunchPadConfigInitialized {
            authority: self.authority,
            asset_rate: self.asset_rate,
            creator_sell_delay: self.creator_sell_delay,
            graduate_threshold: self.graduate_threshold,
            protocol_buy_fee: self.protocol_buy_fee,
            protocol_sell_fee: self.protocol_sell_fee,
            status: self.status,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        // Check protocol is not already paused.
        require!(
            self.status == ProtocolStatus::Active,
            LaunchPadErrorCode::ProtocolAlreadyPaused
        );

        self.status = ProtocolStatus::Paused;
        emit!(LaunchPadPaused {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn unpause(&mut self) -> Result<()> {
        // Check protocol is not already paused.
        require!(
            self.status == ProtocolStatus::Paused,
            LaunchPadErrorCode::ProtocolNotPaused
        );

        self.status = ProtocolStatus::Active;

        emit!(LaunchPadUnpaused {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn calculate_buy_fee(&self, amount: u64) -> Result<u64> {
        let fee = (amount as u128)
            .checked_mul(self.protocol_buy_fee as u128)
            .ok_or(LaunchPadErrorCode::MathOverflow)?
            .checked_div(1_000_000)
            .ok_or(LaunchPadErrorCode::MathOverflow)? as u64;
        Ok(fee)
    }

    pub fn calculate_sell_fee(&self, amount: u64) -> Result<u64> {
        let fee = (amount as u128)
            .checked_mul(self.protocol_sell_fee as u128)
            .ok_or(LaunchPadErrorCode::MathOverflow)?
            .checked_div(1_000_000)
            .ok_or(LaunchPadErrorCode::MathOverflow)? as u64;
        Ok(fee)
    }
}
