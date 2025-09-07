#![allow(deprecated, unexpected_cfgs)]
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::LaunchPadConfig;

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitLaunchPadConfigArgs {
    pub asset_rate: u64,
    pub creator_sell_delay: u64,
    pub graduate_threshold: u64,
    pub protocol_buy_fee: u32,
    pub protocol_sell_fee: u32,
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

    #[account(
        mut,
        seeds = [LaunchPadConfig::VAULT_SEED],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitLaunchPadConfig<'info> {
    pub fn initialize(
        &mut self,
        args: InitLaunchPadConfigArgs,
        bumps: InitLaunchPadConfigBumps,
    ) -> Result<()> {
        self.init_vault()?;
        self.launch_pad_config.initialize(
            self.authority.key(),
            args.asset_rate,
            args.creator_sell_delay,
            args.graduate_threshold,
            args.protocol_buy_fee,
            args.protocol_sell_fee,
            bumps.launch_pad_config,
            bumps.vault,
        )?;
        Ok(())
    }

    fn init_vault(&self) -> Result<()> {
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());

        let cpi_accounts = Transfer {
            from: self.authority.to_account_info(),
            to: self.vault.to_account_info(),
        };
        let cpi_program = self.system_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, rent_exempt)?;
        Ok(())
    }
}

pub fn handler(ctx: Context<InitLaunchPadConfig>, args: InitLaunchPadConfigArgs) -> Result<()> {
    ctx.accounts.initialize(args, ctx.bumps)?;
    msg!("Launch pad config initialized");
    Ok(())
}
