import {
  encodeAbiParameters,
  encodePacked,
  hashMessage,
  keccak256,
  parseAbiParameters,
  recoverMessageAddress,
  toHex,
} from "viem";
import { extractPaymentInfo } from "./utils";
import { privateKeyToAccount } from "viem/accounts";

async function getLog() {
  const log = {
    address: "0x09443ec32e54916366927ccdc9d372474324f427",
    topics: [
      "0xa3162614b8dec8594972fac85313f8db191ab428989960edd147302037f1f2b3",
      "0x0000000000000000000000000000000000000000000000000000000000000001",
      "0x000000000000000000000000898d0dbd5850e086e6c09d2c83a26bb5f1ff8c33",
      "0x00000000000000000000000062c43323447899acb61c18181e34168903e033bf",
    ],
    data: "0x00000000000000000000000019a91e578e2a8117fe4fef93a5cf9ac886efda520000000000000000000000000000000000000000000000000000000000278d00000000000000000000000000036cbd53842c5426634e7929541ec2318f3dcf7e00000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000003e8000000000000000000000000000000000000000000000000000000006759c64e",
    blockHash:
      "0x367d31d79d5938c4295f9015ae5e79e1c26660e4d9bf32fdad165f40e83cfab0",
    blockNumber: "0x12333b7",
    transactionHash:
      "0xd22a22d9674eddc8ee65e62f8643728285ba5abe9d242c07021365be004d6d66",
    transactionIndex: "0x7",
    logIndex: "0x3a",
    removed: false,
  };

  const eventLog = extractPaymentInfo(log);
  console.log(eventLog);
}

getLog();

const privateKey = process.env.WALLET_PRIVATE_KEY as `0x${string}`;
if (!privateKey) {
  throw new Error("Please set WALLET_PRIVATE_KEY in your environment");
}

async function extra() {
  const channelId = BigInt(1);
  const balance = BigInt(999000);
  const nonce = BigInt(1);
  const rawBody = "0x";
  console.log(rawBody);

  const encodedData = encodePacked(
    ["uint256", "uint256", "uint256", "bytes"],
    [channelId, balance, nonce, rawBody]
  );

  console.log(encodedData);
  const messageHash = keccak256(encodedData);
  console.log(messageHash);

  const account = privateKeyToAccount(privateKey);

  const hashedMessage = hashMessage({
    raw: messageHash,
  });

  console.log(hashedMessage);

  const signature = await account.signMessage({
    message: {
      raw: messageHash,
    },
  });

  console.log(signature);

  const address = await recoverMessageAddress({
    message: {
      raw: messageHash,
    },
    signature,
  });

  console.log(address);
}
