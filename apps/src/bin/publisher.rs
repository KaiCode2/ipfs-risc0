// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This application demonstrates how to send an off-chain proof request
// to the Bonsai proving service and publish the received proofs directly
// to your deployed app contract.

use alloy::{
    network::EthereumWallet, providers::ProviderBuilder, signers::local::PrivateKeySigner,
    sol_types::SolValue,
};
use alloy_primitives::{Address, U256};
use anyhow::{Context, Result};
use clap::Parser;
use common::cid::{Attribute, ComputeCid, Player, Skill};
use array_init::array_init;
use methods_player::VERIFY_CID_ELF;
use methods_team::MAKE_TEAM_ELF;
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use risc0_steel::{
    ethereum::{EthEvmEnv, ETH_SEPOLIA_CHAIN_SPEC},
    host::BlockNumberOrTag,
    Commitment, Contract,
};
use tokio::task;
use url::Url;

// `Players` interface automatically generated via the alloy `sol!` macro.
alloy::sol!(
    #[sol(rpc, all_derives)]
    "../contracts/Players.sol"
);

alloy::sol! {
    interface IERC721 {
        function tokenURI(uint256 tokenId) external view returns (string memory uri);
        function ownerOf(uint256 tokenId) external view returns (address owner);
    }

    struct VerifyJournal {
        Commitment commitment;
        address owner;
    }
}

/// Arguments of the publisher CLI.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Ethereum chain ID
    #[clap(long)]
    chain_id: u64,

    /// Ethereum Node endpoint.
    #[clap(long, env = "PRIV_KEY")]
    eth_wallet_private_key: PrivateKeySigner,

    /// Ethereum Node endpoint.
    #[clap(long, env = "RPC_URL_SEPOLIA")]
    rpc_url: Url,

    /// Optional Beacon API endpoint URL
    ///
    /// When provided, Steel uses a beacon block commitment instead of the execution block. This
    /// allows proofs to be validated using the EIP-4788 beacon roots contract.
    #[clap(long, env)]
    beacon_api_url: Option<Url>,

    /// Address of the ERC20 token contract
    #[clap(long, default_value = "ca991c3210075409787fe2a625c22b27fbA098f6")]
    player_contract: Address,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    // Parse CLI Arguments: The application starts by parsing command-line arguments provided by the user.
    let args = Args::parse();

    // Create an alloy provider for that private key and URL.
    let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(args.rpc_url);

    // ABI encode input: Before sending the proof request to the Bonsai proving service,
    // the input number is ABI-encoded to match the format expected by the guest code running in the zkVM.
    // let input = args.input.abi_encode();

    let player = gen_test_player();

    let token_id: U256 = U256::from(0);

    let mut env = EthEvmEnv::builder()
        .provider(provider.clone())
        .block_number_or_tag(BlockNumberOrTag::Parent)
        .build()
        .await?;
    env = env.with_chain_spec(&ETH_SEPOLIA_CHAIN_SPEC);

    let mut contract = Contract::preflight(args.player_contract, &mut env);
    let owner_call = IERC721::ownerOfCall {
        tokenId: U256::from(token_id),
    };
    let uri_call = IERC721::tokenURICall {
        tokenId: U256::from(token_id),
    };
    let owner_result =  contract.call_builder(&owner_call).call().await?;
    let uri_result = contract.call_builder(&uri_call).call().await?;

    println!("Owner: {:?}", owner_result.owner);
    println!("URI: {:?}", uri_result.uri);
    println!("Player CID: {:?}", player.formatted_cid());

    let evm_input = if let Some(beacon_api_url) = args.beacon_api_url {
        #[allow(deprecated)]
        env.into_beacon_input(beacon_api_url).await?
    } else {
        env.into_input().await?
    };
    let cloned_evm_input = evm_input.clone();

    let prove_info = task::spawn_blocking(move || {
        let env = ExecutorEnv::builder()
            .write(&evm_input)?
            .write(&player)?
            .write(&token_id)?
            .build()
            .unwrap();

        default_prover().prove_with_ctx(
            env,
            &VerifierContext::default(),
            VERIFY_CID_ELF,
            &ProverOpts::groth16(),
        )
    })
    .await?
    .context("failed to create CID verification proof")?;
    let receipt = prove_info.receipt;
    let journal = &receipt.journal.bytes;

    // Decode and log the commitment
    let journal = VerifyJournal::abi_decode(journal, true).context("invalid journal")?;
    log::debug!("Steel commitment: {:?}", journal.commitment);

    // ABI encode the seal.
    let seal = encode_seal(&receipt).context("invalid receipt")?;

    println!("Journal owner: {:?}", journal.owner);

    let players: [Player; 11] = array_init(|_| gen_test_player());
    let token_ids: [U256; 11] = [token_id; 11];

    let make_team_proof = task::spawn_blocking(move || {
        let env = ExecutorEnv::builder()
            .write(&cloned_evm_input)?
            .write(&journal.owner)?
            .write(&players)?
            .write(&token_ids)?
            .add_assumption(receipt)
            .build()
            .unwrap();

        default_prover().prove_with_ctx(
            env,
            &VerifierContext::default(),
            MAKE_TEAM_ELF,
            &ProverOpts::groth16(),
        )
    }).await?
    .context("failed to make team create proof")?;

    let receipt = make_team_proof.receipt;
    let journal = &receipt.journal.bytes;


    Ok(())
}

fn gen_test_player() -> Player {
    Player {
        name: "Lionel Messi".to_string(),
        jersey_number: 10,
        description: "A professional footballer who plays as a forward for Paris Saint-Germain and the Argentina national team.".to_string(),
        external_url: "https://en.wikipedia.org/wiki/Lionel_Messi".to_string(),
        image: "https://upload.wikimedia.org/wikipedia/commons/4/47/Lionel_Messi_20180626.jpg".to_string(),
        tier: 1,
        overall_rating: 94.0,
        skill_multiplier: 1.0,
        skill: Skill {
            speed: 90,
            shooting: 95,
            passing: 90,
            dribbling: 96,
            defense: 32,
            physical: 68,
            goal_tending: 0,
        },
        attributes: vec![
            Attribute {
                display_type: "Physical".to_string(),
                trait_type: "Height".to_string(),
                value: 170.0,
            },
            Attribute {
                display_type: "Physical".to_string(),
                trait_type: "Weight".to_string(),
                value: 72.0,
            },
        ],
    }
}
