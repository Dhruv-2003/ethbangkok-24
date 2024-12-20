use ethers::{
    abi::{encode_packed, Tokenizable},
    types::{Bytes, U256},
    utils::keccak256,
};

pub fn create_message(channel_id: U256, balance: U256, nonce: U256, body: &[u8]) -> Vec<u8> {
    let message = encode_packed(&[
        channel_id.into_token(),
        balance.into_token(),
        nonce.into_token(),
        Bytes::from(body.to_vec()).into_token(),
    ])
    .unwrap();

    keccak256(message).to_vec()
}
