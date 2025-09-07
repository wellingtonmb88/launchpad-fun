use anchor_lang::prelude::*;

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default, InitSpace,
)]
pub enum ProtocolStatus {
    #[default]
    Unknown,
    Active,
    Paused,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default, InitSpace,
)]
pub enum LaunchPadTokenStatus {
    #[default]
    Unknown,
    TradingEnabled,
    ReadyToGraduate,
    Graduated,
}
