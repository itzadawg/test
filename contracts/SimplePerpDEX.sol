// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {MockPriceOracle} from "./MockPriceOracle.sol";

/// @title SimplePerpDEX
/// @notice Educational perpetual DEX core logic for prototyping and workshops.
contract SimplePerpDEX {
    struct Position {
        bool isLong;
        uint256 margin; // collateral locked in the position (in wei)
        uint256 size; // notional exposure (in wei)
        uint256 entryPrice; // oracle price when position was opened
    }

    uint256 public constant BPS_DIVISOR = 10_000;

    MockPriceOracle public immutable oracle;
    address public immutable feeCollector;

    uint256 public feeBps = 10; // 0.10% default maker/taker fee

    mapping(address => uint256) public balances;
    mapping(address => Position) public positions;

    event Deposited(address indexed account, uint256 amount);
    event Withdrawn(address indexed account, uint256 amount);
    event PositionOpened(
        address indexed account,
        bool isLong,
        uint256 margin,
        uint256 size,
        uint256 entryPrice
    );
    event PositionClosed(
        address indexed account,
        uint256 payout,
        int256 pnl,
        uint256 feePaid,
        uint256 closePrice
    );
    event FeeUpdated(uint256 newFeeBps);

    error InsufficientBalance();
    error PositionExists();
    error NoOpenPosition();
    error InvalidAmount();
    error TransferFailed();
    error Unauthorized();

    constructor(MockPriceOracle oracle_, address feeCollector_) {
        oracle = oracle_;
        feeCollector = feeCollector_;
    }

    // --- Collateral management ---

    function deposit() external payable {
        if (msg.value == 0) {
            revert InvalidAmount();
        }
        balances[msg.sender] += msg.value;
        emit Deposited(msg.sender, msg.value);
    }

    function withdraw(uint256 amount) external {
        if (amount == 0) {
            revert InvalidAmount();
        }
        Position storage position = positions[msg.sender];
        if (position.size != 0) {
            revert PositionExists();
        }
        uint256 available = balances[msg.sender];
        if (available < amount) {
            revert InsufficientBalance();
        }
        balances[msg.sender] = available - amount;
        (bool success, ) = msg.sender.call{value: amount}("");
        if (!success) {
            revert TransferFailed();
        }
        emit Withdrawn(msg.sender, amount);
    }

    // --- Trading ---

    function openPosition(bool isLong, uint256 notional, uint256 leverage) external {
        if (notional == 0 || leverage == 0) {
            revert InvalidAmount();
        }
        Position storage position = positions[msg.sender];
        if (position.size != 0) {
            revert PositionExists();
        }

        uint256 marginRequired = notional / leverage;
        require(marginRequired > 0, "Margin too small");

        uint256 available = balances[msg.sender];
        if (available < marginRequired) {
            revert InsufficientBalance();
        }

        uint256 currentPrice = oracle.getPrice();
        balances[msg.sender] = available - marginRequired;
        positions[msg.sender] = Position({
            isLong: isLong,
            margin: marginRequired,
            size: notional,
            entryPrice: currentPrice
        });

        emit PositionOpened(msg.sender, isLong, marginRequired, notional, currentPrice);
    }

    function closePosition() external {
        Position storage position = positions[msg.sender];
        if (position.size == 0) {
            revert NoOpenPosition();
        }

        uint256 currentPrice = oracle.getPrice();
        int256 pnl = _calculatePnl(position, currentPrice);

        uint256 payout = position.margin;
        if (pnl > 0) {
            payout += uint256(pnl);
        } else {
            uint256 loss = uint256(-pnl);
            if (loss >= payout) {
                payout = 0;
            } else {
                payout -= loss;
            }
        }

        uint256 fee = (position.size * feeBps) / BPS_DIVISOR;
        if (fee > payout) {
            fee = payout;
        }

        balances[msg.sender] += payout - fee;
        delete positions[msg.sender];

        if (fee > 0) {
            (bool success, ) = feeCollector.call{value: fee}("");
            if (!success) {
                revert TransferFailed();
            }
        }

        emit PositionClosed(msg.sender, payout, pnl, fee, currentPrice);
    }

    function updateFeeBps(uint256 newFeeBps) external {
        if (msg.sender != feeCollector) {
            revert Unauthorized();
        }
        require(newFeeBps <= 100, "Fee too high");
        feeBps = newFeeBps;
        emit FeeUpdated(newFeeBps);
    }

    // --- View helpers ---

    function getAccountValue(address account) external view returns (uint256 equity, int256 pnl) {
        Position memory position = positions[account];
        uint256 balance = balances[account];
        if (position.size == 0) {
            return (balance, 0);
        }
        uint256 currentPrice = oracle.getPrice();
        int256 currentPnl = _calculatePnl(position, currentPrice);
        int256 total = int256(balance + position.margin) + currentPnl;
        if (total < 0) {
            return (0, currentPnl);
        }
        return (uint256(total), currentPnl);
    }

    function hasOpenPosition(address account) external view returns (bool) {
        return positions[account].size != 0;
    }

    function _calculatePnl(Position memory position, uint256 currentPrice) internal pure returns (int256) {
        if (position.size == 0 || position.entryPrice == 0) {
            return 0;
        }
        int256 priceDelta = int256(currentPrice) - int256(position.entryPrice);
        int256 pnl = (priceDelta * int256(position.size)) / int256(position.entryPrice);
        if (!position.isLong) {
            pnl = -pnl;
        }
        return pnl;
    }

    receive() external payable {
        deposit();
    }
}
