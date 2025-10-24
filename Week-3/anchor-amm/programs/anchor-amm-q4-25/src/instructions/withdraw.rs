use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    //The LPer who want to burn the LP tokens and get back the investment of both the tokens that they invested previously.
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        has_one= mint_x,
        has_one = mint_y,
        seeds=[b"config", config.seed.to_le_bytes().as_ref()],
        bump=config.config_bump
    )]
    pub config :Account<'info, Config>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config
    )]
    pub vault_y: Account<'info, TokenAccount>,

    //Lp token PDA
    #[account(
        mut, 
        seeds=[b"lp", config.key().as_ref()],
        bump=config.lp_bump
    )]
    pub mint_lp: Account<'info, Mint>,

    //User x and y ATA
    #[account(
        mut,
        associated_token::mint=mint_x,
        associated_token::authority=user
    )]
    pub user_x: Account<'info,TokenAccount>,

    #[account(
        mut,
        associated_token::mint=mint_y,
        associated_token::authority=user
    )]
    pub user_y: Account<'info,TokenAccount>,

    pub user_lp: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        amount: u64, // Amount of LP tokens that the user wants to "burn"
        min_x: u64,  // Minimum amount of token X that the user wants to receive
        min_y: u64,  // Minimum amount of token Y that the user wants to receive
    ) -> Result<()> {
        //Check if the config is locked or not.
        //And the amount of liquidity tokens must not be 0.
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);

        //First calculate the amount of x and y tokens that the user may receive in exchange of the LP token.
        let xy_amount = ConstantProduct::xy_withdraw_amounts_from_l(self.vault_x.amount, self.vault_y.amount, self.mint_lp.supply, amount, 6).unwrap();
        let x_tokens = xy_amount.x;
        let y_tokens = xy_amount.y;

        //Check that we are satisfying that the slippage is not exceeding.
        require!(x_tokens >=min_x && y_tokens >=min_y, AmmError::SlippageExceeded);

        //If slippage is not exceeding, then burn the LP tokens.
        self.burn_lp_tokens(amount)?;
        //Withdraw the x_tokens from the vault and transfer it to the user.
        self.withdraw_tokens(true, x_tokens)?;
        //Withdraw the y_tokens from the vault and transfer it to the user.
        self.withdraw_tokens(false, y_tokens)
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        //TODO
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info() 
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info()
            )
            
        };
        //Define the accounts involved in the transfer
        let accounts = Transfer{
            from, 
            to,
            authority: self.config.to_account_info()
        };

        //create the cpi_context
        let cpi_context = CpiContext::new(self.token_program.to_account_info(), accounts);
        
        transfer(cpi_context, amount)
    }


    pub fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        //TODO
        let accounts = Burn{
            mint: self.mint_lp.to_account_info(),
            from: self.user_lp.to_account_info(),
            authority: self.config.to_account_info()
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump]
            ]];
        let cpi_context = CpiContext::new_with_signer(self.token_program.to_account_info(), accounts, signer_seeds);
        burn(cpi_context, amount)
    }
}
