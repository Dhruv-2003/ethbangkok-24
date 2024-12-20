import { createPublicClient, createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { baseSepolia } from "viem/chains";
import { paymentChannelABI } from "../abi/abi";

const privateKey = process.env.WALLET_PRIVATE_KEY as `0x${string}`;
if (!privateKey) {
  throw new Error("Please provide a private key");
}

// This is for API Providers, to close the channel and withdraw the remaining balance in payment channel
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

  const paymentChannel = {
    address: "0xc72dfac1a7b3bc178f10dc3bf36c7f64cf41b7de",
    sender: "0x898d0dbd5850e086e6c09d2c83a26bb5f1ff8c33",
    recipient: "0x62c43323447899acb61c18181e34168903e033bf",
    balance: "999000",
    nonce: "1",
    expiration: "1736528914",
    channel_id: "2",
  };

  const rawBody = "0x";
  const signature =
    "0x7158589b32a76eef73c612a0848f6ba66e6a9bf0592414fc5dbd374b185c7114507061d26bcf88f01d68fc7ea18bedb89c564103830b1f7a37e89055a8ebab4d1b"; // Signature of the sender

  const txHash = await walletClient.writeContract({
    abi: paymentChannelABI,
    address: paymentChannel.address as `0x${string}`,
    functionName: "close",
    args: [
      BigInt(paymentChannel.balance),
      BigInt(paymentChannel.nonce),
      rawBody,
      signature,
    ],
  });

  await publicClient.waitForTransactionReceipt({
    hash: txHash,
  });
}

main();
