# Solana simple-perp-dex program

This directory contains an Anchor-based Solana implementation of the same perpetual exchange primitives that exist in the EVM `SimplePerpDEX` contract. The program keeps custody of trader deposits in a PDA-owned vault, tracks margin requirements, collects fees, and emits rich events so off-chain services can mirror portfolio state.

## Layout

```text
solana/
├── Anchor.toml                    # Anchor configuration for localnet/devnet builds
├── Cargo.toml                     # Workspace manifest sharing anchor-lang dependencies
├── programs/
│   └── simple-perp-dex/
│       ├── Cargo.toml             # Crate manifest for the on-chain program
│       └── src/lib.rs             # Program logic and account definitions
└── README.md
```

## Building and testing

1. Install the Anchor CLI (version `0.29.x`) and Solana tool suite if you have not already:

   ```bash
   cargo install --git https://github.com/coral-xyz/anchor avm --locked
   avm use 0.29.0
   avm install 0.29.0
   solana-install init 1.17.15
   ```

2. Start a local test validator in a separate terminal:

   ```bash
   solana-test-validator
   ```

3. Build and deploy the program locally:

   ```bash
   cd solana
   anchor build
   anchor deploy
   ```

   The `simple-perp-dex` program uses PDAs to hold collateral (`vault`) and trader state. After deployment you can airdrop SOL to test traders and exercise the instructions via Anchor tests or custom scripts.

4. (Optional) Generate the IDL and TypeScript client:

   ```bash
   anchor build --idl
   ```

   The resulting `target/idl/simple_perp_dex.json` file can be consumed by Anchor client code or SDKs.

## Instruction summary

| Instruction      | Purpose                                                                 | Key accounts                                                                 |
| ---------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `initialize`     | Creates global configuration, fee collector, and collateral vault PDAs. | `global_state`, `vault`, authority signer, fee collector wallet             |
| `deposit`        | Moves SOL from the trader into the program vault and updates balance.    | `trader_state` PDA (auto-created), vault PDA, trader signer                 |
| `withdraw`       | Releases SOL back to the trader once no positions are open.              | `trader_state`, vault PDA (signing via seeds), trader signer                |
| `open_position`  | Locks margin, records position direction/size, and caches entry price.   | `trader_state`, trader signer                                               |
| `close_position` | Calculates PnL, pays fees to the collector, restores collateral balance. | `trader_state`, vault PDA, fee collector, trader signer, system program     |
| `update_fee`     | Lets the authority adjust maker/taker fees (capped at 1%).               | `global_state`, authority signer                                            |

The pricing math mirrors the Solidity implementation: notional values are expressed with the same 8-decimal convention, and fees are applied in basis points against position size.

## Integrating with the TypeScript toolchain

- Generate client bindings: `anchor client gen ./target/idl/simple_perp_dex.json --lang ts --out-dir ../sdk` (or point your own SDK to the IDL file).
- Re-use the repository's `scripts/relayPythPrice.ts` helper to broadcast live Pyth prices from Solana into the EVM `MockPriceOracle` so both deployments stay aligned during demos.
- Manage trader onboarding by deriving their PDA using the seed tuple `['trader', trader_pubkey, global_state_pubkey]`. The PDA bump is stored on-chain to simplify client derivation.

## Next steps

- Swap the mock vault with an SPL token vault to margin trades in USDC instead of SOL.
- Extend the program with keeper-managed liquidation hooks that can mirror the EVM contract's automation.
- Wire an Anchor test suite that mirrors the Hardhat scenarios to guarantee identical behaviour across chains.
