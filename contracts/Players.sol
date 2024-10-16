// SPDX-License-Identifier: MIT

pragma solidity ^0.8.20;

import { ERC721EnumerableURI } from "./extensions/ERC721EnumerableURI.sol";
import { ERC721 } from "@openzeppelin/contracts/token/ERC721/ERC721.sol";

contract Players is ERC721EnumerableURI {

    constructor() ERC721("Player", "PLYR") {}

    function mint(uint256 tokenId, bytes32 hash) public {
        _mint(msg.sender, tokenId, hash);
    }
}
