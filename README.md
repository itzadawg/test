# Perp DEX Starter Kit

Launch a minimal perpetual decentralized exchange (perp DEX) in minutes. This repository bundles a battle-tested Hardhat + TypeScript toolchain, a composable core smart contract, a mock oracle, and deployment scripts so that teams can start iterating on perp mechanics without wrestling with boilerplate.

## Features

- **Upgradeable architecture** – modular Solidity contracts (`SimplePerpDEX` + `MockPriceOracle`) that can be extended or replaced as your design evolves.
- **Opinionated risk controls** – isolated margin accounting, configurable fees, and simple PnL calculation logic that serve as a foundation for backtesting new ideas.
- **One-command local environment** – Hardhat network for deterministic testing and scripts to spin up or reset state quickly.
- **Type-safe tooling** – TypeScript-based tests, deployment scripts, and configuration ready for rapid customization.
- **Environment ready for production** – `.env` management, Sepolia network configuration, and CI-friendly commands (`build`, `test`, `lint`).

## Repository structure

```text
contracts/              # Solidity source (SimplePerpDEX + MockPriceOracle)
scripts/deploy.ts       # Hardhat deployment entrypoint
test/SimplePerpDEX.test.ts  # End-to-end scenario tests
hardhat.config.ts       # Hardhat + network configuration
.env.example            # Template for environment variables
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
   # Populate SEPOLIA_RPC_URL and DEPLOYER_KEY when targeting public testnets
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

## Customizing the DEX

- **Risk parameters** – adjust `feeBps`, extend margin logic, or add liquidation hooks inside `SimplePerpDEX.sol`.
- **Oracle integrations** – replace `MockPriceOracle` with a Chainlink adapter or custom Pyth connector.
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
