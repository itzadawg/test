import { expect } from "chai";
import { ethers } from "hardhat";
import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";

const PRICE_DECIMALS = 8;

function price(value: string) {
  return ethers.parseUnits(value, PRICE_DECIMALS);
}

describe("SimplePerpDEX", function () {
  async function deployFixture() {
    const [deployer, feeCollector, trader] = await ethers.getSigners();

    const oracle = await ethers.deployContract("MockPriceOracle", [price("2000")]);
    await oracle.waitForDeployment();

    const dex = await ethers.deployContract("SimplePerpDEX", [await oracle.getAddress(), feeCollector.address]);
    await dex.waitForDeployment();

    return { dex, oracle, deployer, feeCollector, trader };
  }

  it("records deposits", async function () {
    const { dex, trader } = await deployFixture();

    const depositAmount = ethers.parseEther("5");
    await expect(dex.connect(trader).deposit({ value: depositAmount }))
      .to.emit(dex, "Deposited")
      .withArgs(trader.address, depositAmount);

    expect(await dex.balances(trader.address)).to.equal(depositAmount);
  });

  it("realises profit for a winning long position", async function () {
    const { dex, oracle, trader, feeCollector } = await deployFixture();

    const depositAmount = ethers.parseEther("1.5");
    await dex.connect(trader).deposit({ value: depositAmount });

    const notional = ethers.parseEther("5");
    const leverage = 5n;

    await dex.connect(trader).openPosition(true, notional, leverage);

    const feeCollectorBalanceBefore = await ethers.provider.getBalance(feeCollector.address);

    await oracle.setPrice(price("2200"));

    const closeTx = await dex.connect(trader).closePosition();
    await expect(closeTx)
      .to.emit(dex, "PositionClosed")
      .withArgs(trader.address, anyValue, anyValue, anyValue, price("2200"));

    const balance = await dex.balances(trader.address);
    const fee = (notional * 10n) / 10_000n; // default feeBps is 10
    const expectedPnl = (price("2200") - price("2000")) * notional / price("2000");
    const expectedBalance = depositAmount + expectedPnl - fee; // leftover margin already counted in depositAmount

    expect(balance).to.equal(expectedBalance);

    const feeCollectorBalanceAfter = await ethers.provider.getBalance(feeCollector.address);
    expect(feeCollectorBalanceAfter - feeCollectorBalanceBefore).to.equal(fee);

    expect(await dex.hasOpenPosition(trader.address)).to.equal(false);
  });

  it("registers loss for a short position", async function () {
    const { dex, oracle, trader } = await deployFixture();

    const depositAmount = ethers.parseEther("1");
    await dex.connect(trader).deposit({ value: depositAmount });

    const notional = ethers.parseEther("4");
    const leverage = 4n;

    await dex.connect(trader).openPosition(false, notional, leverage);

    await oracle.setPrice(price("2100"));

    await dex.connect(trader).closePosition();

    const balance = await dex.balances(trader.address);
    expect(balance).to.be.lessThan(depositAmount);
  });

  it("blocks withdrawals while a position is open", async function () {
    const { dex, trader } = await deployFixture();

    await dex.connect(trader).deposit({ value: ethers.parseEther("1") });
    await dex.connect(trader).openPosition(true, ethers.parseEther("3"), 3n);

    await expect(dex.connect(trader).withdraw(ethers.parseEther("0.5"))).to.be.revertedWithCustomError(
      dex,
      "PositionExists"
    );
  });
});
