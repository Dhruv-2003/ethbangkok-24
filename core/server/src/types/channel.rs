use ethers::types::{Address, Signature, U256};
// use alloy::{
//     primitives::{Address, U256},
//     signers::Signature,
// };
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentChannel {
    pub address: Address,
    pub sender: Address,
    pub recipient: Address,

    // #[serde_as(as = "DisplayFromStr")]
    #[serde(
        deserialize_with = "deserialize_u256_from_str",
        serialize_with = "serialize_u256_as_str"
    )]
    pub balance: U256,

    // #[serde_as(as = "DisplayFromStr")]
    #[serde(
        deserialize_with = "deserialize_u256_from_str",
        serialize_with = "serialize_u256_as_str"
    )]
    pub nonce: U256,

    // #[serde_as(as = "DisplayFromStr")]
    #[serde(
        deserialize_with = "deserialize_u256_from_str",
        serialize_with = "serialize_u256_as_str"
    )]
    pub expiration: U256,

    // #[serde_as(as = "DisplayFromStr")]
    #[serde(
        deserialize_with = "deserialize_u256_from_str",
        serialize_with = "serialize_u256_as_str"
    )]
    pub channel_id: U256,
}

fn serialize_u256_as_str<S>(x: &U256, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.collect_str(&x)
}

fn deserialize_u256_from_str<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(U256::from_dec_str(&s).map_err(serde::de::Error::custom)?)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedRequest {
    pub message: Vec<u8>,
    pub signature: Signature,
    pub payment_channel: PaymentChannel,
    pub payment_amount: U256,
    pub body_bytes: Vec<u8>,
}
