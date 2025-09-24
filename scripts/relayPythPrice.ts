import * as dotenv from "dotenv";
import { parsePriceData } from "@pythnetwork/client";
import { Connection, PublicKey } from "@solana/web3.js";
import hre from "hardhat";

dotenv.config();

const TARGET_DECIMALS = 8;

function scalePrice(rawPrice: number, exponent: number, targetDecimals: number): bigint {
  const base = BigInt(Math.round(rawPrice));
  const scale = targetDecimals + exponent;

  if (scale === 0) {
    return base;
  }

  const absScale = BigInt(Math.abs(scale));
  const factor = BigInt(10) ** absScale;

  if (scale > 0) {
    return base * factor;
  }

  if (base % factor === BigInt(0)) {
    return base / factor;
  }

  console.warn("Warning: truncating precision while scaling Pyth price to target decimals");
  const half = factor / BigInt(2);
  const adjusted = base >= BigInt(0) ? base + half : base - half;
  return adjusted / factor;
}

async function main() {
  const solanaRpcUrl = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
  const priceAccountAddress = process.env.PYTH_PRICE_ACCOUNT;
  const oracleAddress = process.env.MOCK_ORACLE_ADDRESS;
  const relayerKey = process.env.RELAYER_PRIVATE_KEY;

  if (!priceAccountAddress) {
    throw new Error("Missing PYTH_PRICE_ACCOUNT in environment");
  }

  if (!oracleAddress) {
    throw new Error("Missing MOCK_ORACLE_ADDRESS in environment");
  }

  const connection = new Connection(solanaRpcUrl, "confirmed");
  const priceAccount = new PublicKey(priceAccountAddress);
  const accountInfo = await connection.getAccountInfo(priceAccount);

  if (!accountInfo) {
    throw new Error(`Price account ${priceAccountAddress} not found`);
  }

  const priceData = parsePriceData(accountInfo.data);
  const price = priceData.price ?? priceData.previousPrice;

  if (price === undefined) {
    throw new Error("Price feed has not reported a value yet");
  }

  const scaledPrice = scalePrice(price, priceData.exponent, TARGET_DECIMALS);
  const displayPrice = price * Math.pow(10, priceData.exponent);
  const { ethers } = hre;

  const provider = ethers.provider;
  let signer = (await ethers.getSigners())[0];

  if (relayerKey) {
    signer = new ethers.Wallet(relayerKey, provider);
  }

  const oracle = await ethers.getContractAt("MockPriceOracle", oracleAddress, signer);
  const tx = await oracle.setPrice(scaledPrice);
  await tx.wait();

  console.log(
    `Relayed Pyth price ${displayPrice} (raw ${price} * 10^${priceData.exponent}) as ${scaledPrice.toString()} to oracle ${oracleAddress}`
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
