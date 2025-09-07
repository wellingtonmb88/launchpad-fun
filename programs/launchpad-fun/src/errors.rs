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

    #[msg("Invalid creator")]
    InvalidCreator,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("LaunchPadToken not created")]
    LaunchPadTokenNotCreated,

    #[msg("LaunchPadToken already created")]
    LaunchPadTokenAlreadyCreated,

    #[msg("LaunchPadToken already graduated")]
    LaunchPadTokenAlreadyGraduated,

    #[msg("Invalid token name length")]
    InvalidTokenNameLength,

    #[msg("Invalid token symbol length")]
    InvalidTokenSymbolLength,

    #[msg("Invalid token URI length")]
    InvalidTokenUriLength,

    #[msg("Math overflow")]
    MathOverflow,
}
