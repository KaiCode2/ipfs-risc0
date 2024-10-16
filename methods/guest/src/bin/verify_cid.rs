use std::io::Read;

use common::cid::{Player, ComputeCid};
use alloy_primitives::{U256, Address, address};
use alloy_sol_types::{sol, SolValue};
use risc0_zkvm::guest::env;
use risc0_steel::{ethereum::{EthEvmInput, ETH_SEPOLIA_CHAIN_SPEC}, Contract, Commitment};

risc0_zkvm::guest::entry!(main);

sol! {
    interface IERC721 {
        function tokenURI(uint256 tokenId) external view returns (string memory uri);
        function ownerOf(uint256 tokenId) external view returns (address owner);
    }

    struct Journal {
        Commitment commitment;
        address owner;
    }
}

pub const PLAYER_CONTRACT_ADDRESS: Address = address!("ca991c3210075409787fe2a625c22b27fbA098f6");

fn main() {
    let chain_config: EthEvmInput = env::read();
    let player: Player = env::read();
    let token_id: U256 = env::read();

    let env = chain_config
        .into_env()
        .with_chain_spec(&ETH_SEPOLIA_CHAIN_SPEC);

    let contract = Contract::new(PLAYER_CONTRACT_ADDRESS, &env);

    let owner_call = IERC721::ownerOfCall {
        tokenId: U256::from(token_id),
    };
    let owner = contract.call_builder(&owner_call).call().owner;

    let player_cid_call = IERC721::tokenURICall {
        tokenId: U256::from(token_id),
    };
    let player_cid = contract.call_builder(&player_cid_call).call().uri;

    let expected_cid = player.formatted_cid();
    assert!(
        expected_cid == player_cid,
        "Player CID does not match on-chain data"
    );

    let journal = Journal {
        commitment: env.into_commitment(),
        owner,
    };

    env::commit_slice(&journal.abi_encode());
}
