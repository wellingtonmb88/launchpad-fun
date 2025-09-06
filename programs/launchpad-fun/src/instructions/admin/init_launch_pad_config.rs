#![allow(deprecated, unexpected_cfgs)]
use anchor_lang::prelude::*;

use crate::LaunchPadConfig;

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitLaunchPadConfigArgs {
    pub asset_rate: u64,
    pub creator_sell_delay: u64,
    pub graduate_threshold: u64,
    pub protocol_fee: u32,
}

#[derive(Accounts)]
pub struct InitLaunchPadConfig<'info> {
    // The admin authority that is initializing launch pad config.
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = LaunchPadConfig::DISCRIMINATOR.len() + LaunchPadConfig::INIT_SPACE,
        seeds = [LaunchPadConfig::SEED],
        bump
    )]
    pub launch_pad_config: Account<'info, LaunchPadConfig>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitLaunchPadConfig<'info> {
    pub fn initialize(&mut self, args: InitLaunchPadConfigArgs, bump: u8) -> Result<()> {
        self.launch_pad_config.initialize(
            self.authority.key(),
            args.asset_rate,
            args.creator_sell_delay,
            args.graduate_threshold,
            args.protocol_fee,
            bump,
        )?;
        Ok(())
    }
}

pub fn handler(ctx: Context<InitLaunchPadConfig>, args: InitLaunchPadConfigArgs) -> Result<()> {
    ctx.accounts.initialize(args, ctx.bumps.launch_pad_config)?;
    msg!("Launch pad config initialized");
    Ok(())
}
