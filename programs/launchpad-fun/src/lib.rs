#![allow(deprecated, unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;
pub mod statuses;

pub use constants::*;
pub use errors::*;
pub use events::*;
pub use instructions::*;
pub use math::*;
pub use state::*;
pub use statuses::*;

declare_id!("HqY2bef2WwBtVSLJhii8GJ2aG3wFgDNECHYHc6Y1zHkR");

#[program]
pub mod launchpad_fun {
    use super::*;

    pub fn initialize(
        ctx: Context<InitLaunchPadConfig>,
        args: InitLaunchPadConfigArgs,
    ) -> Result<()> {
        init_launch_pad_config::handler(ctx, args)?;
        Ok(())
    }

    pub fn create_token(ctx: Context<CreateToken>, args: CreateTokenArgs) -> Result<()> {
        create_token::handler(ctx, args)?;
        Ok(())
    }

    pub fn buy_token(ctx: Context<BuyToken>, args: BuyTokenArgs) -> Result<()> {
        buy_token::handler(ctx, args)?;
        Ok(())
    }
}
