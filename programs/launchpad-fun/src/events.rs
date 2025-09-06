use anchor_lang::prelude::*;

use crate::{LaunchPadTokenStatus, ProtocolStatus};

#[event]
#[derive(Debug)]
pub struct LaunchPadConfigInitialized {
    pub authority: Pubkey,
    pub asset_rate: u64,
    pub creator_sell_delay: u64,
    pub graduate_threshold: u64,
    pub protocol_fee: u32,
    pub status: ProtocolStatus,
    pub timestamp: i64,
}

#[event]
#[derive(Debug)]
pub struct LaunchPadPaused {
    pub timestamp: i64,
}

#[event]
#[derive(Debug)]
pub struct LaunchPadUnpaused {
    pub timestamp: i64,
}

#[event]
#[derive(Debug)]
pub struct LaunchPadTokenCreated {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub status: LaunchPadTokenStatus,
    pub timestamp: i64,
}

#[event]
#[derive(Debug)]
pub struct LaunchPadTokenGraduated {
    pub mint: Pubkey,
    pub status: LaunchPadTokenStatus,
    pub timestamp: i64,
}
