use std::time::{SystemTime, UNIX_EPOCH};

use alloy::{hex, primitives::U256, signers::Signature};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::{
    channel::ChannelState,
    error::AuthError,
    types::{PaymentChannel, SignedRequest},
    utils::create_message,
};

pub async fn auth_middleware(
    state: ChannelState,
    payment_amount: U256, // defined by the developer creating the API, and should match with what user agreed with in the signed request
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // parse the request to retrieve the required headers
    // Check timestamp first
    let timestamp = request
        .headers()
        .get("X-Timestamp")
        .and_then(|t| t.to_str().ok())
        .and_then(|t| t.parse::<u64>().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    println!("Timestamp: {}", timestamp);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now - timestamp > 300 {
        return Err(StatusCode::REQUEST_TIMEOUT);
    }

    // Get and validate all required headers
    let signature = request
        .headers()
        .get("X-Signature")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let message = request
        .headers()
        .get("X-Message")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let payment_data = request
        .headers()
        .get("X-Payment")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Print all the headers
    println!("Signature: {}", signature);
    println!("Message: {}", message);
    println!("Payment Data: {}", payment_data);

    // Parse signature
    let signature = hex::decode(signature.trim_start_matches("0x"))
        .map_err(|_| {
            println!("Failed: Signature decode");
            StatusCode::BAD_REQUEST
        })
        .and_then(|bytes| {
            Signature::try_from(bytes.as_slice()).map_err(|_| {
                println!("Failed: Signature conversion");
                StatusCode::BAD_REQUEST
            })
        })?;

    // Parse message
    let message = hex::decode(message).map_err(|_| {
        println!("Failed: Message decode");
        StatusCode::BAD_REQUEST
    })?;

    // Parse payment channel data
    let mut payment_channel: PaymentChannel = serde_json::from_str(payment_data).map_err(|_| {
        println!("Failed: Payment data decode");
        StatusCode::BAD_REQUEST
    })?;

    // Get request body
    let (parts, body) = request.into_parts();
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            println!("Failed: Body decode");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Verify that the message matches what we expect
    let reconstructed_message = create_message(
        payment_channel.channel_id,
        payment_channel.balance,
        payment_channel.nonce,
        &body_bytes,
    );

    // Update Balance for updating the local state
    payment_channel.balance -= payment_amount;

    if message != reconstructed_message {
        println!("Failed: Message mismatch");
        return Err(StatusCode::BAD_REQUEST);
    }

    let signed_request = SignedRequest {
        message: message.clone(),
        signature,
        payment_channel: payment_channel,
        payment_amount,
    };

    // Validate the headers against the payment channel state and return the response
    match verify_and_update_channel(&state, &signed_request).await {
        Ok(payment_channel) => {
            let request = Request::from_parts(parts, Body::from(body_bytes));

            // Modify the response headers to include the payment channel data
            let mut response = next.run(request).await;
            let headers_mut = response.headers_mut();
            // convert the payment channel json into string and then return that in the header
            headers_mut.insert(
                "X-Payment",
                serde_json::to_string(&payment_channel)
                    .unwrap()
                    .parse()
                    .unwrap(),
            );
            headers_mut.insert("X-Timestamp", now.to_string().parse().unwrap());

            Ok(response)
        }
        Err(e) => Err(StatusCode::from(e)),
    }
}

async fn verify_and_update_channel(
    state: &ChannelState,
    request: &SignedRequest,
) -> Result<PaymentChannel, AuthError> {
    println!("\n=== verify_and_update_channel ===");
    println!("Payment amount: {}", request.payment_amount);
    println!("Channel balance: {}", request.payment_channel.balance);

    println!("Message length: {}", request.message.len());
    println!("Original message: 0x{}", hex::encode(&request.message));

    // Verify signature using network-specific logic
    let recovered_address = state
        .verify_signature(
            request.payment_channel.channel_id,
            &request.signature,
            &request.message,
        )
        .await?;

    if recovered_address != request.payment_channel.sender {
        println!("Failed: Address mismatch");
        return Err(AuthError::InvalidSignature);
    }

    state
        .check_rate_limit(request.payment_channel.sender)
        .await?;

    let mut channels = state.channels.write().await;

    // Check if channel exists and validate nonce
    if let Some(existing_channel) = channels.get(&request.payment_channel.channel_id) {
        // Ensure new nonce is greater than existing nonce
        if request.payment_channel.nonce <= existing_channel.nonce {
            println!(
                "Failed: Invalid nonce - current: {}, received: {}",
                existing_channel.nonce, request.payment_channel.nonce
            );
            return Err(AuthError::InvalidChannel);
        }
    } else {
        // TODO: Implement the check here

        // Validate channel balance using network-specific logic
        state
            .validate_channel(request.payment_channel.channel_id, &request.payment_channel)
            .await?;

        // Handle the case where the channel does not exist
        // Ensure the nonce is 0
        // Verify that the channel contract data is correct
        // 1. Verify the balance is available in the contract as the channel balance
        // 2. Verify the expiration is in the future
        // 3. Verify the channel ID is correct
    }

    // Update or insert the channel
    channels.insert(
        request.payment_channel.channel_id,
        request.payment_channel.clone(),
    );

    Ok(request.payment_channel.clone())
}
