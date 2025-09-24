// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title MockPriceOracle
/// @notice Simple manually updatable oracle used for local development and testing.
contract MockPriceOracle {
    address public immutable owner;
    uint256 private _price;

    event PriceUpdated(uint256 newPrice);

    error NotOwner();

    constructor(uint256 initialPrice) {
        owner = msg.sender;
        _price = initialPrice;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) {
            revert NotOwner();
        }
        _;
    }

    function setPrice(uint256 newPrice) external onlyOwner {
        _price = newPrice;
        emit PriceUpdated(newPrice);
    }

    function getPrice() external view returns (uint256) {
        return _price;
    }
}
