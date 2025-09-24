use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};
use std::convert::TryFrom;

declare_id!("PerpDex1111111111111111111111111111111111");

const BPS_DIVISOR: u64 = 10_000;

#[program]
pub mod simple_perp_dex {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        require!(fee_bps <= 100, DexError::FeeTooHigh);

        let global = &mut ctx.accounts.global_state;
        global.authority = ctx.accounts.authority.key();
        global.fee_collector = ctx.accounts.fee_collector.key();
        global.fee_bps = fee_bps;
        global.vault_bump = *ctx.bumps.get("vault").unwrap();

        emit!(DexInitialized {
            authority: global.authority,
            fee_collector: global.fee_collector,
            fee_bps,
        });

        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, DexError::InvalidAmount);

        let trader_state = &mut ctx.accounts.trader_state;
        let trader = ctx.accounts.trader.key();

        if trader_state.owner == Pubkey::default() {
            trader_state.owner = trader;
            trader_state.bump = *ctx.bumps.get("trader_state").unwrap();
        } else {
            require!(trader_state.owner == trader, DexError::Unauthorized);
        }

        let transfer_accounts = Transfer {
            from: ctx.accounts.trader.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        system_program::transfer(cpi_ctx, amount)?;

        trader_state.balance = trader_state
            .balance
            .checked_add(amount)
            .ok_or(DexError::MathOverflow)?;

        emit!(Deposited {
            trader,
            amount,
            new_balance: trader_state.balance,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, DexError::InvalidAmount);

        let trader_state = &mut ctx.accounts.trader_state;
        let trader = ctx.accounts.trader.key();

        require!(trader_state.owner == trader, DexError::Unauthorized);
        require!(!trader_state.has_position, DexError::PositionExists);
        require!(
            trader_state.balance >= amount,
            DexError::InsufficientBalance
        );

        trader_state.balance -= amount;

        let seeds = &[
            b"vault",
            ctx.accounts.global_state.key().as_ref(),
            &[ctx.accounts.global_state.vault_bump],
        ];
        let transfer_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.trader.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
            &[seeds],
        );
        system_program::transfer(cpi_ctx, amount)?;

        emit!(Withdrawn {
            trader,
            amount,
            remaining_balance: trader_state.balance,
        });

        Ok(())
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        is_long: bool,
        notional: u64,
        leverage: u64,
        oracle_price: i64,
    ) -> Result<()> {
        require!(notional > 0 && leverage > 0, DexError::InvalidAmount);
        require!(oracle_price > 0, DexError::InvalidAmount);

        let trader_state = &mut ctx.accounts.trader_state;
        let trader = ctx.accounts.trader.key();

        require!(trader_state.owner == trader, DexError::Unauthorized);
        require!(!trader_state.has_position, DexError::PositionExists);

        let margin_required = notional
            .checked_div(leverage)
            .ok_or(DexError::InvalidAmount)?;
        require!(margin_required > 0, DexError::InvalidAmount);
        require!(
            trader_state.balance >= margin_required,
            DexError::InsufficientBalance
        );

        trader_state.balance -= margin_required;
        trader_state.position_margin = margin_required;
        trader_state.position_size = notional;
        trader_state.entry_price = oracle_price;
        trader_state.is_long = is_long;
        trader_state.has_position = true;

        emit!(PositionOpened {
            trader,
            is_long,
            margin: margin_required,
            size: notional,
            entry_price: oracle_price,
        });

        Ok(())
    }

    pub fn close_position(ctx: Context<ClosePosition>, oracle_price: i64) -> Result<()> {
        require!(oracle_price > 0, DexError::InvalidAmount);

        let global_state = &ctx.accounts.global_state;
        let trader_state = &mut ctx.accounts.trader_state;
        let trader = ctx.accounts.trader.key();

        require!(trader_state.owner == trader, DexError::Unauthorized);
        require!(trader_state.has_position, DexError::NoOpenPosition);

        let pnl = calculate_pnl(
            trader_state.position_size,
            trader_state.entry_price,
            oracle_price,
            trader_state.is_long,
        )?;

        let mut payout: i128 = trader_state.position_margin as i128;
        if pnl >= 0 {
            payout = payout.checked_add(pnl).ok_or(DexError::MathOverflow)?;
        } else {
            let loss = (-pnl) as i128;
            payout = payout.checked_sub(loss).unwrap_or(0);
        }

        let mut fee: u64 = trader_state
            .position_size
            .checked_mul(global_state.fee_bps as u64)
            .ok_or(DexError::MathOverflow)?
            / BPS_DIVISOR;

        if (fee as i128) > payout {
            fee = u64::try_from(payout).map_err(|_| DexError::MathOverflow)?;
        }

        let net_payout = payout
            .checked_sub(fee as i128)
            .ok_or(DexError::MathOverflow)?;

        let net_payout_u64 = u64::try_from(net_payout).map_err(|_| DexError::MathOverflow)?;

        trader_state.balance = trader_state
            .balance
            .checked_add(net_payout_u64)
            .ok_or(DexError::MathOverflow)?;

        trader_state.position_margin = 0;
        trader_state.position_size = 0;
        trader_state.entry_price = 0;
        trader_state.is_long = false;
        trader_state.has_position = false;

        if fee > 0 {
            let seeds = &[
                b"vault",
                ctx.accounts.global_state.key().as_ref(),
                &[ctx.accounts.global_state.vault_bump],
            ];
            let transfer_accounts = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.fee_collector.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                transfer_accounts,
                &[seeds],
            );
            system_program::transfer(cpi_ctx, fee)?;
        }

        emit!(PositionClosed {
            trader,
            payout: net_payout_u64,
            pnl,
            fee_paid: fee,
            close_price: oracle_price,
        });

        Ok(())
    }

    pub fn update_fee(ctx: Context<UpdateFee>, new_fee_bps: u16) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.global_state.authority,
            DexError::Unauthorized
        );
        require!(new_fee_bps <= 100, DexError::FeeTooHigh);

        ctx.accounts.global_state.fee_bps = new_fee_bps;

        emit!(FeeUpdated {
            authority: ctx.accounts.authority.key(),
            new_fee_bps,
        });

        Ok(())
    }
}

fn calculate_pnl(size: u64, entry_price: i64, close_price: i64, is_long: bool) -> Result<i128> {
    if size == 0 || entry_price == 0 {
        return Ok(0);
    }

    let price_delta = i128::from(close_price) - i128::from(entry_price);
    let pnl = price_delta
        .checked_mul(i128::from(size))
        .ok_or(DexError::MathOverflow)?
        .checked_div(i128::from(entry_price))
        .ok_or(DexError::MathOverflow)?;

    if is_long {
        Ok(pnl)
    } else {
        Ok(-pnl)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = GlobalState::LEN)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: stored as part of global state for fee withdrawals
    pub fee_collector: UncheckedAccount<'info>,
    #[account(
        init,
        payer = authority,
        seeds = [b"vault", global_state.key().as_ref()],
        bump,
        space = 0
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    pub global_state: Account<'info, GlobalState>,
    #[account(
        mut,
        seeds = [b"vault", global_state.key().as_ref()],
        bump = global_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        init_if_needed,
        payer = trader,
        space = TraderState::LEN,
        seeds = [b"trader", trader.key().as_ref(), global_state.key().as_ref()],
        bump
    )]
    pub trader_state: Account<'info, TraderState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    pub global_state: Account<'info, GlobalState>,
    #[account(
        mut,
        seeds = [b"vault", global_state.key().as_ref()],
        bump = global_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"trader", trader.key().as_ref(), global_state.key().as_ref()],
        bump = trader_state.bump
    )]
    pub trader_state: Account<'info, TraderState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    pub global_state: Account<'info, GlobalState>,
    #[account(
        mut,
        seeds = [b"trader", trader.key().as_ref(), global_state.key().as_ref()],
        bump = trader_state.bump
    )]
    pub trader_state: Account<'info, TraderState>,
}

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        mut,
        seeds = [b"trader", trader.key().as_ref(), global_state.key().as_ref()],
        bump = trader_state.bump
    )]
    pub trader_state: Account<'info, TraderState>,
    #[account(
        mut,
        seeds = [b"vault", global_state.key().as_ref()],
        bump = global_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    /// CHECK: constrained to the collector stored in global state
    #[account(mut, address = global_state.fee_collector)]
    pub fee_collector: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateFee<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
}

#[account]
pub struct GlobalState {
    pub authority: Pubkey,
    pub fee_collector: Pubkey,
    pub fee_bps: u16,
    pub vault_bump: u8,
    pub _padding: [u8; 5],
}

impl GlobalState {
    pub const LEN: usize = 8 + 32 + 32 + 2 + 1 + 5;
}

#[account]
pub struct TraderState {
    pub owner: Pubkey,
    pub balance: u64,
    pub position_margin: u64,
    pub position_size: u64,
    pub entry_price: i64,
    pub is_long: bool,
    pub has_position: bool,
    pub bump: u8,
    pub _padding: [u8; 5],
}

impl TraderState {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 5;
}

#[event]
pub struct DexInitialized {
    pub authority: Pubkey,
    pub fee_collector: Pubkey,
    pub fee_bps: u16,
}

#[event]
pub struct Deposited {
    pub trader: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
}

#[event]
pub struct Withdrawn {
    pub trader: Pubkey,
    pub amount: u64,
    pub remaining_balance: u64,
}

#[event]
pub struct PositionOpened {
    pub trader: Pubkey,
    pub is_long: bool,
    pub margin: u64,
    pub size: u64,
    pub entry_price: i64,
}

#[event]
pub struct PositionClosed {
    pub trader: Pubkey,
    pub payout: u64,
    pub pnl: i128,
    pub fee_paid: u64,
    pub close_price: i64,
}

#[event]
pub struct FeeUpdated {
    pub authority: Pubkey,
    pub new_fee_bps: u16,
}

#[error_code]
pub enum DexError {
    #[msg("Invalid amount supplied")]
    InvalidAmount,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Position already exists")]
    PositionExists,
    #[msg("No open position")]
    NoOpenPosition,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Fee too high")]
    FeeTooHigh,
}
