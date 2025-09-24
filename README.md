# Perp DEX Starter Kit

Launch a minimal perpetual decentralized exchange (perp DEX) in minutes. This repository bundles a battle-tested Hardhat + TypeScript toolchain, a composable core smart contract, a mock oracle, and deployment scripts so that teams can start iterating on perp mechanics without wrestling with boilerplate.

## Features

- **Upgradeable architecture** – modular Solidity contracts (`SimplePerpDEX` + `MockPriceOracle`) that can be extended or replaced as your design evolves.
- **Opinionated risk controls** – isolated margin accounting, configurable fees, and simple PnL calculation logic that serve as a foundation for backtesting new ideas.
- **One-command local environment** – Hardhat network for deterministic testing and scripts to spin up or reset state quickly.
- **Type-safe tooling** – TypeScript-based tests, deployment scripts, and configuration ready for rapid customization.
- **Environment ready for production** – `.env` management, Sepolia network configuration, and CI-friendly commands (`build`, `test`, `lint`).
- **Cross-chain parity** – matching Anchor program and Solana tooling so you can prototype on EVM and Solana simultaneously.

## Repository structure

```text
contracts/              # Solidity source (SimplePerpDEX + MockPriceOracle)
scripts/deploy.ts       # Hardhat deployment entrypoint
scripts/relayPythPrice.ts   # Relays Pyth price data from Solana to the mock oracle
test/SimplePerpDEX.test.ts  # End-to-end scenario tests
hardhat.config.ts       # Hardhat + network configuration
.env.example            # Template for environment variables
solana/                 # Anchor workspace with the Simple Perp DEX Solana program
```

## Getting started

1. **Install dependencies**

   ```bash
   npm install
   ```

   > _Offline environments_: if npm registry access is restricted, install dependencies in a machine with access and copy the resulting `node_modules` directory or use a private mirror.

2. **Bootstrap environment variables**

   ```bash
   cp .env.example .env
   # Populate SEPOLIA_RPC_URL/DEPLOYER_KEY for EVM deployments and
   # SOLANA_RPC_URL/PYTH_PRICE_ACCOUNT/MOCK_ORACLE_ADDRESS/RELAYER_PRIVATE_KEY
   # when relaying Solana prices into the mock oracle.
   ```

3. **Compile contracts**

   ```bash
   npm run build
   ```

4. **Run the automated test suite**

   ```bash
   npm test
   ```

   The tests simulate profitable and unprofitable positions, verify fee routing, and enforce margin checks.

5. **Lint Solidity sources** (optional but recommended)

   ```bash
   npm run lint
   ```

## Quick local trading session

1. Start a local Hardhat node (in a separate terminal):

   ```bash
   npx hardhat node
   ```

2. Deploy contracts to the local network:

   ```bash
   npx hardhat run scripts/deploy.ts --network localhost
   ```

3. Interact with the DEX via Hardhat console or scripts, for example:

   ```bash
   npx hardhat console --network localhost
   > const [trader] = await ethers.getSigners();
   > const dex = await ethers.getContractAt("SimplePerpDEX", "<deployed_address>");
   > await dex.connect(trader).deposit({ value: ethers.parseEther("5") });
   > await dex.connect(trader).openPosition(true, ethers.parseEther("10"), 5n);
   ```

## Deploying to Sepolia (or another EVM chain)

1. Ensure `.env` contains a funded private key and RPC endpoint.
2. Compile contracts with `npm run build`.
3. Deploy:

   ```bash
   npx hardhat run scripts/deploy.ts --network sepolia
   ```

   The script prints deployed addresses for both the mock oracle and the exchange. Swap in a production oracle when you are ready to launch beyond local environments.

## Solana simple-perp-dex program

The `solana/` workspace mirrors the Solidity contracts with an Anchor-based program that manages margin accounts, collateral custody, and fee routing on Solana. Follow the dedicated [Solana README](./solana/README.md) to install Anchor, build the program, and deploy it to a local validator or devnet. Both deployments share identical position math and event semantics so you can port strategies across chains without rewriting business logic.

Key design notes:

- Trader state lives in PDAs derived from `['trader', trader_pubkey, global_state_pubkey]`, making it straightforward for clients to discover accounts.
- Collateral is held in a vault PDA owned by the program, enabling safe withdrawals only when no positions are open.
- Fees are capped at 1% (100 bps) and routed to the fee collector wallet supplied during initialization.

## Relaying Solana prices into the EVM mock oracle

Use the `scripts/relayPythPrice.ts` helper whenever you want the Hardhat deployment to follow live Pyth price feeds:

```bash
npx hardhat run scripts/relayPythPrice.ts --network localhost
```

Configuration is pulled from `.env`:

- `SOLANA_RPC_URL` – RPC endpoint for devnet/localnet/mainnet.
- `PYTH_PRICE_ACCOUNT` – the Pyth price account public key (e.g., BTC/USD on devnet).
- `MOCK_ORACLE_ADDRESS` – address of the `MockPriceOracle` you want to update.
- `RELAYER_PRIVATE_KEY` – optional EVM key used to submit the transaction (defaults to the first Hardhat signer).

The script fetches the latest price, normalizes it to the oracle's 8-decimal format, and calls `setPrice` so your Solidity and Solana environments reference the same market data.

## Customizing the DEX

- **Risk parameters** – adjust `feeBps`, extend margin logic, or add liquidation hooks inside `SimplePerpDEX.sol`.
- **Oracle integrations** – replace `MockPriceOracle` with a Chainlink adapter or the included Solana Pyth relay script for live market data.
- **UI integration** – the contract exposes simple view helpers (`getAccountValue`, `hasOpenPosition`) that front-ends can query to display portfolio state.
- **Automation** – integrate Gelato or keepers by adding roles that can trigger `closePosition` when margin falls below thresholds.

## Extending the toolkit

- Add a frontend (Next.js / wagmi) that talks to the deployed contracts.
- Wire CI to run `npm run build` and `npm test` on pull requests.
- Package reusable SDK bindings by generating TypeChain typings (`npx hardhat typechain`).

## Troubleshooting

- **Dependency installation issues** – point npm to a mirror (`npm config set registry <url>`) or vendor dependencies.
- **Gas errors on close** – ensure the exchange contract holds enough ETH to pay out PnL and fees (deposit extra funds when testing extreme profits).
- **Oracle precision mismatch** – `MockPriceOracle` assumes 8 decimal places. Align external feeds or adjust the conversion helpers in your scripts/tests accordingly.

## License

MIT – feel free to use, fork, and adapt.
