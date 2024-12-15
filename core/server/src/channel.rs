// Channel struct and implementation
// It's the local channel state for the middleware on the server side on how to store the info and just work with it

use std::{
    // collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use alloy::{
    contract::Error,
    network::EthereumWallet,
    primitives::{Address, FixedBytes, U256},
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signature},
    sol,
};
use alloy::{primitives::Bytes, transports::http::reqwest::Url};
use dashmap::DashMap;

// #[cfg(not(target_arch = "wasm32"))]
// use tokio::sync::RwLock;

// #[cfg(target_arch = "wasm32")]
// use std::sync::RwLock;

use crate::{error::AuthError, types::PaymentChannel};

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PaymentChannelContract,
    "src/abi/PaymentChannel.json"
);

#[derive(Clone)]
pub struct ChannelState {
    // #[cfg(not(target_arch = "wasm32"))]
    // pub(crate) channels: Arc<RwLock<HashMap<U256, PaymentChannel>>>, // All the channels the current server has with other user

    // #[cfg(target_arch = "wasm32")]
    // pub(crate) channels: Arc<RwLock<HashMap<U256, PaymentChannel>>>,
    pub(crate) channels: Arc<DashMap<U256, PaymentChannel>>,

    // rate_limiter: Arc<RwLock<HashMap<Address, (u64, SystemTime)>>>, // Rate limiter for the user
    rate_limiter: Arc<DashMap<Address, (u64, SystemTime)>>,
    network_rpc_url: Url, // provider: Arc<dyn Provider>, // Provider to interact with the blockchain
}

impl ChannelState {
    pub fn new(rpc_url: Url) -> Self {
        Self {
            // channels: Arc::new(RwLock::new(HashMap::new())),
            // rate_limiter: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(DashMap::new()),
            rate_limiter: Arc::new(DashMap::new()),
            network_rpc_url: rpc_url,
        }
    }

    pub fn get_channel(&self, channel_id: U256) -> Option<PaymentChannel> {
        // let channels = self.channels.read().await;
        // channels.get(&channel_id).cloned()
        self.channels
            .get(&channel_id)
            .map(|entry| entry.value().clone())
    }

    pub fn update_channel(&self, channel_id: U256, channel: PaymentChannel) {
        self.channels.insert(channel_id, channel);
    }

    // verification method

    pub async fn verify_signature(
        &self,
        payment_channel: &PaymentChannel,
        signature: &Signature,
        message: &[u8],
    ) -> Result<(), AuthError> {
        // self.network.verify_signature(signature, message).await

        // Network logic to verify the signature, could be a simple ECDSA verification
        // TODO: Recheck this logic
        let recovered = signature.recover_address_from_msg(message);
        println!("Recovered address: {:?}", recovered);

        // Match the recovered address with the one in the channel state
        match recovered {
            Ok(address) if address == payment_channel.sender => Ok(()),
            _ => {
                Err(AuthError::InvalidSignature)
                // NOTE : Ok(Address::default())
            }
        }
    }

    // Validating all the information of the channel from the onchain contract for the first time, before the channel is used
    pub async fn validate_channel(
        &self,
        payment_channel: &PaymentChannel,
    ) -> Result<(), AuthError> {
        // self.network.validate_channel(channel_id, balance).await
        let provider = ProviderBuilder::new().on_http(self.network_rpc_url.clone());

        let payment_channel_contract =
            PaymentChannelContract::new(payment_channel.address, provider);

        let balance_value = payment_channel_contract
            .getBalance()
            .call()
            .await
            .unwrap()
            ._0;

        let balance = U256::from(balance_value);

        println!("Balance: {}", balance);

        // If the balance is less than the balance in the local state, return an error
        if payment_channel.balance < balance {
            return Err(AuthError::InsufficientBalance);
        }

        let expiration_value = payment_channel_contract
            .expiration()
            .call()
            .await
            .unwrap()
            ._0;

        let expiration = U256::from(expiration_value);

        println!("Expiration: {}", expiration);

        if payment_channel.expiration != expiration {
            return Err(AuthError::Expired);
        }

        // Verify the channelID from the contract
        let channel_id_value = payment_channel_contract
            .channelId()
            .call()
            .await
            .unwrap()
            ._0;
        let channel_id = U256::from(channel_id_value);

        println!("Channel ID: {}", channel_id);

        if payment_channel.channel_id != channel_id {
            return Err(AuthError::InvalidChannel);
        }

        // Verify sender and recipient from the contract
        let sender_value = payment_channel_contract.sender().call().await.unwrap()._0;

        if payment_channel.sender != sender_value {
            return Err(AuthError::InvalidChannel);
        }

        let recipient_value = payment_channel_contract
            .recipient()
            .call()
            .await
            .unwrap()
            ._0;

        if payment_channel.recipient != recipient_value {
            return Err(AuthError::InvalidChannel);
        }

        Ok(())
    }

    // rate limiter method
    // ✅
    pub(crate) async fn check_rate_limit(&self, sender: Address) -> Result<(), AuthError> {
        const RATE_LIMIT: u64 = 100; // 100 requests
        const WINDOW: u64 = 60; // Every 60 seconds

        // let mut rate_limits = self.rate_limiter.write().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // let (count, last_reset) = rate_limits.entry(sender).or_insert((0, SystemTime::now()));

        // let last_reset_secs = last_reset.duration_since(UNIX_EPOCH).unwrap().as_secs();

        // if now - last_reset_secs >= WINDOW {
        //     *count = 1;
        //     *last_reset = SystemTime::now();
        //     Ok(())
        // } else if *count >= RATE_LIMIT {
        //     Err(AuthError::RateLimitExceeded)
        // } else {
        //     *count += 1;
        //     Ok(())
        // }

        let mut entry = self
            .rate_limiter
            .entry(sender)
            .or_insert_with(|| (0, SystemTime::now()));

        let last_reset_secs = entry.1.duration_since(UNIX_EPOCH).unwrap().as_secs();
        if now - last_reset_secs >= WINDOW {
            // Reset the count and timestamp
            entry.0 = 1;
            entry.1 = SystemTime::now();
            Ok(())
        } else if entry.0 >= RATE_LIMIT {
            // Exceeded rate limit
            Err(AuthError::RateLimitExceeded)
        } else {
            // Increment the count
            entry.0 += 1;
            Ok(())
        }
    }
}

// Close the channel to withdraw the funds
pub async fn close_channel(
    rpc_url: Url,
    private_key: &str,
    payment_channel: &PaymentChannel,
    signature: &Signature,
    raw_body: Bytes,
) -> Result<FixedBytes<32>, Error> {
    let signer: PrivateKeySigner = private_key.parse().expect("Invalid private key");
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(rpc_url.clone());

    let payment_channel_contract = PaymentChannelContract::new(payment_channel.address, provider);

    let tx_hash = payment_channel_contract
        .close(
            payment_channel.balance,
            payment_channel.nonce,
            raw_body,
            Bytes::from(signature.as_bytes()),
        )
        .send()
        .await?
        .watch()
        .await?;

    Ok(tx_hash)
}
