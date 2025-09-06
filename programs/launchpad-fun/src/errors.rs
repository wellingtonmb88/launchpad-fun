use anchor_lang::prelude::*;

#[error_code]
pub enum LaunchPadErrorCode {
    #[msg("Invalid authority")]
    InvalidAuthority,

    #[msg("ProtocolConfig already initialized")]
    ProtocolConfigInitialized,

    #[msg("ProtocolConfig not initialized")]
    ProtocolConfigNotInitialized,

    #[msg("ProtocolConfig not active")]
    ProtocolConfigNotActive,

    #[msg("Protocol is already paused")]
    ProtocolAlreadyPaused,

    #[msg("Protocol is not paused")]
    ProtocolNotPaused,

    #[msg("Creator sell delay not met")]
    CreatorSellDelayNotMet,

    #[msg("Asset rate must be greater than zero")]
    AssetRateMustBeGreaterThanZero,

    #[msg("Graduate threshold not met")]
    GraduateThresholdNotMet,

    #[msg("Protocol fee exceeds maximum")]
    ProtocolFeeExceedsMaximum,

    #[msg("Protocol fee minimum not met")]
    ProtocolFeeMinimumNotMet,
}
