#!/bin/bash

# NOTE: This script requires the `cast` CLI tool to be installed.


# Constants
RPC_URL="https://base-sepolia-rpc.publicnode.com"
PRIVATE_KEY=""  # Replace with your private key , Ensure the account has enough balance to create a channel
TOKEN_ADDRESS="0x036CbD53842c5426634e7929541eC2318f3dCF7e" # USDC
CHANNEL_FACTORY_ADDRESS="0x09443Ec32E54916366927ccDC9D372474324F427"
RECIPIENT="0x62C43323447899acb61C18181e34168903E033Bf"
AMOUNT="1000000"  # Token amount in 10^6 (e.g., 1 USDC)
DURATION="2592000"  # Duration in seconds (e.g., 30 days)

# Step 1: Approve stablecoin to the Channel Factory
echo "Approving stablecoin to the Channel Factory..."
cast send $USDC_ADDRESS \
    "approve(address spender, uint256 value)" \
    $CHANNEL_FACTORY_ADDRESS $AMOUNT \
    --rpc-url $RPC_URL \
    --private-key $PRIVATE_KEY

if [ $? -ne 0 ]; then
    echo "Failed to approve stablecoin. Exiting."
    exit 1
fi
echo "Stablecoin approved successfully."

# Step 2: Create a payment channel
echo "Creating a payment channel..."
TRANSACTION_HASH=$(cast send "$CHANNEL_FACTORY" \
  "createChannel(address recipient, uint256 _duration,address _tokenAddress, uint256 _amount)" \
  "$RECIPIENT" "$DURATION" "$TOKEN_ADDRESS" "$AMOUNT" \
  --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --json | jq -r '.transactionHash')

if [ $? -ne 0 ]; then
    echo "Failed to create payment channel. Exiting."
    exit 1
fi
echo "Payment channel created successfully."

# Step 3: Fetch transaction logs
echo "Fetching transaction logs..."
LOGS=$(cast tx "$TRANSACTION_HASH" --rpc-url "$RPC_URL" --json)

# Step 4: Extract relevant log
echo "Extracting relevant log..."
LOG=$(echo "$LOGS" | jq -c '.logs[] | select(.topics[0] == "0xa3162614b8dec8594972fac85313f8db191ab428989960edd147302037f1f2b3")')

# Step 5: Decode and parse log data
echo "Decoding log data..."
DATA=$(echo "$LOG" | jq -r '.data')
SENDER=$(echo "$LOG" | jq -r '.topics[2]')
RECIPIENT=$(echo "$LOG" | jq -r '.topics[3]')
BALANCE=$(echo "$DATA" | cut -c3-66 | xargs printf "%d\n")
NONCE=$(echo "$DATA" | cut -c67-130 | xargs printf "%d\n")
EXPIRATION=$(echo "$DATA" | cut -c131-194 | xargs printf "%d\n")
PRICE=$(echo "$DATA" | cut -c195-258 | xargs printf "%d\n")


# Step 6: Fetch balance for the payment channel
echo "Fetching balance for the payment channel..."
BALANCE=$(cast call $CHANNEL_ADDRESS \
    "getBalance()" \
    --rpc-url $RPC_URL)

if [ $? -ne 0 ]; then
    echo "Failed to fetch balance. Exiting."
    exit 1
fi
echo "Payment channel balance: $BALANCE"

# Step 7: Create JSON output
echo "Creating payment.json..."
cat <<EOF > payment.json
{
  "address": "$TOKEN_ADDRESS",
  "sender": "0x${SENDER:26}",
  "recipient": "0x${RECIPIENT:26}",
  "balance": $BALANCE,
  "nonce": $NONCE,
  "expiration": $EXPIRATION,
  "channel_id": 1,
  "price": $PRICE
}
EOF

echo "Payment information saved to payment.json"