import { ethers } from "hardhat";

async function main() {
  const [deployer, feeCollector] = await ethers.getSigners();

  console.log("Deploying contracts with", deployer.address);

  const initialPrice = ethers.parseUnits("2000", 8);

  const oracle = await ethers.deployContract("MockPriceOracle", [initialPrice]);
  await oracle.waitForDeployment();
  console.log("MockPriceOracle deployed at", await oracle.getAddress());

  const dex = await ethers.deployContract("SimplePerpDEX", [await oracle.getAddress(), feeCollector.address]);
  await dex.waitForDeployment();
  console.log("SimplePerpDEX deployed at", await dex.getAddress());

  console.log("Fee collector set to", feeCollector.address);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
