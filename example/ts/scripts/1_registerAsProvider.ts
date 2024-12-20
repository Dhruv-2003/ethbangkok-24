import { createPublicClient, createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { baseSepolia } from "viem/chains";
import { channelFactoryAbi, paymentChannelABI } from "../abi/abi";
import { ChannelFactoryAddress } from "../constants";

const privateKey = process.env.WALLET_PRIVATE_KEY as `0x${string}`;
if (!privateKey) {
  throw new Error("Please provide a private key");
}

// This script is for API providers, to register themselves as a service provider and set the price
async function main() {
  const account = privateKeyToAccount(privateKey);

  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });

  const walletClient = createWalletClient({
    chain: baseSepolia,
    transport: http(),
    account,
  });

  const price = BigInt(1000); // 0.001 USDC

  const txHash = await walletClient.writeContract({
    abi: channelFactoryAbi,
    address: ChannelFactoryAddress,
    functionName: "register",
    args: [price],
  });

  await publicClient.waitForTransactionReceipt({
    hash: txHash,
  });
}
