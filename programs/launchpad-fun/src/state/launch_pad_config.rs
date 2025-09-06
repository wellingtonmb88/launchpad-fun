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
    pub authority: Pubkey,
    pub asset_rate: u64,
    pub creator_sell_delay: u64,
    pub graduate_threshold: u64,
    pub protocol_fee: u32,
    pub status: ProtocolStatus,
    pub bump: u8,
}

impl LaunchPadConfig {
    pub const SEED: &'static [u8] = b"launch_pad_config:";

    pub fn initialize(
        &mut self,
        authority: Pubkey,
        asset_rate: u64,
        creator_sell_delay: u64,
        graduate_threshold: u64,
        protocol_fee: u32,
        bump: u8,
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
            self.creator_sell_delay > time as u64,
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
            protocol_fee <= MAX_PROTOCOL_FEE,
            LaunchPadErrorCode::ProtocolFeeExceedsMaximum
        );
        require!(
            protocol_fee >= MIN_PROTOCOL_FEE,
            LaunchPadErrorCode::ProtocolFeeMinimumNotMet
        );
        self.authority = authority;
        self.asset_rate = asset_rate;
        self.creator_sell_delay = creator_sell_delay;
        self.graduate_threshold = graduate_threshold;
        self.protocol_fee = protocol_fee;
        self.status = ProtocolStatus::Active;
        self.bump = bump;

        emit!(LaunchPadConfigInitialized {
            authority: self.authority,
            asset_rate: self.asset_rate,
            creator_sell_delay: self.creator_sell_delay,
            graduate_threshold: self.graduate_threshold,
            protocol_fee: self.protocol_fee,
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
}
