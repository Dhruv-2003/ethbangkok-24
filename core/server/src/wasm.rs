use alloy::{hex, primitives::U256, signers::Signature};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::{
    channel::ChannelState,
    types::{PaymentChannel, SignedRequest},
    verify::verify_and_update_channel,
};

#[wasm_bindgen]
pub struct WasmChannelState {
    inner: ChannelState,
}

#[wasm_bindgen]
impl WasmChannelState {
    #[wasm_bindgen(constructor)]
    pub fn new(rpc_url: &str) -> Result<WasmChannelState, JsError> {
        let url = rpc_url
            .parse()
            .map_err(|e| JsError::new(&format!("Invalid URL: {}", e)))?;

        Ok(WasmChannelState {
            inner: ChannelState::new(url),
        })
    }

    #[wasm_bindgen]
    pub fn verify_request(
        &self,
        message: String,
        signature: String,
        payment_channel_json: String,
        payment_amount: u64,
        body_bytes: Vec<u8>,
    ) -> js_sys::Promise {
        let state = self.inner.clone();

        future_to_promise(async move {
            let message: Vec<u8> = unhexlify(&message)
                .map_err(|e| JsValue::from_str(&format!("Invalid request: {}", e)))?;

            let signature: Signature = unhexlify(&signature)
                .map_err(|e| JsValue::from_str(&format!("Invalid signature: {}", e)))
                .and_then(|bytes| {
                    Signature::try_from(bytes.as_slice())
                        .map_err(|_| JsValue::from_str("Invalid signature: invalid length"))
                })?;

            let payment_channel: PaymentChannel = serde_json::from_str(&payment_channel_json)
                .map_err(|e| JsValue::from_str(&format!("Invalid payment channel: {}", e)))?;

            let payment_amount = U256::from(payment_amount);

            let request = SignedRequest {
                message,
                signature,
                payment_channel,
                payment_amount,
                body_bytes,
            };

            let result = verify_and_update_channel(&state, request)
                .await
                .map_err(|e| JsValue::from_str(&format!("Verification failed: {}", e)))?;

            Ok(JsValue::from_str(&serde_json::to_string(&result).unwrap()))
        })
    }
}

fn hexlify(a: &[u8]) -> String {
    let mut output = "0x".to_owned();
    output.push_str(&hex::encode(a));

    output
}

fn unhexlify(h: &String) -> Result<Vec<u8>, hex::FromHexError> {
    let mut prefix = h.to_owned();
    let s = prefix.split_off(2);
    let result = hex::decode(&s);

    result
}
