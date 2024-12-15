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
    println!("\n=== auth_middleware ===");
    println!(" === new request ===");

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
    let payment_channel: PaymentChannel = serde_json::from_str(payment_data).map_err(|e| {
        println!("Failed: Payment data decode - Error {}", e);
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
    println!("Body: {}", String::from_utf8_lossy(&body_bytes));

    // Verify that the message matches what we expect
    let reconstructed_message = create_message(
        payment_channel.channel_id,
        payment_channel.balance,
        payment_channel.nonce,
        &body_bytes,
    );

    if message != reconstructed_message {
        println!("Failed: Message mismatch");
        return Err(StatusCode::BAD_REQUEST);
    } else {
        println!("Message match");
    }

    let signed_request = SignedRequest {
        message,
        signature,
        payment_channel,
        payment_amount,
        body_bytes: body_bytes.to_vec(),
    };

    // Validate the headers against the payment channel state and return the response
    match verify_and_update_channel(&state, signed_request).await {
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

            println!(" === end request ===\n");

            Ok(response)
        }
        Err(e) => Err(StatusCode::from(e)),
    }
}

async fn verify_and_update_channel(
    state: &ChannelState,
    mut request: SignedRequest,
) -> Result<PaymentChannel, AuthError> {
    println!("\n=== verify_and_update_channel ===");
    println!("Payment amount: {}", request.payment_amount);
    println!(
        "Payment channel: {}",
        serde_json::to_string(&request.payment_channel).unwrap()
    );
    println!("Channel balance: {}", request.payment_channel.balance);

    println!("Message length: {}", request.message.len());
    println!("Original message: 0x{}", hex::encode(&request.message));

    // 1. Verify signature using network-specific logic
    state
        .verify_signature(
            &request.payment_channel,
            &request.signature,
            &request.message,
        )
        .await?;

    // 2. Check for rate limiting
    state
        .check_rate_limit(request.payment_channel.sender)
        .await?;

    // let mut channels = state.channels.write().await;
    let exisisting_channels = state.get_channel(request.payment_channel.channel_id);

    // Check if channel exists
    // NOTE: Nonce validation can be skipped as the balance will be acting as nonce here, the sender will always send the tx with the highest balance, we'll check for that here within our local record
    if let Some(existing_channel) = exisisting_channels {
        println!("Existing channel found");
        // Ensure new nonce is greater than existing nonce
        if request.payment_channel.nonce <= existing_channel.nonce {
            println!(
                "Failed: Invalid nonce - current: {}, received: {}",
                existing_channel.nonce, request.payment_channel.nonce
            );
            return Err(AuthError::InvalidChannel);
        } else {
            println!("Nonce match");
        }

        if request.payment_channel.balance != existing_channel.balance {
            println!(
                "Failed: Invalid balance - current: {}, received: {}",
                existing_channel.balance, request.payment_channel.balance
            );
            return Err(AuthError::InvalidChannel);
        } else {
            println!("Balance match");
        }
    } else {
        println!("New channel found");

        // Verify that the channel contract data is correct
        // 1. Verify the balance is available in the contract as the channel balance
        // 2. Verify the expiration is in the future
        // 3. Verify the channel ID is correct
        state.validate_channel(&request.payment_channel).await?;

        // Ensure the nonce is 0
        if request.payment_channel.nonce != U256::from(0) {
            return Err(AuthError::InvalidChannel);
        }
    }

    // NOTE: Update Balance for updating the local state, deducting the balance from the channel
    println!("Updating channel state");
    request.payment_channel.balance -= request.payment_amount;

    // Update or insert the channel
    state.update_channel(
        request.payment_channel.channel_id,
        request.payment_channel.clone(),
    );

    println!("API request authorized");
    Ok(request.payment_channel.clone())
}
