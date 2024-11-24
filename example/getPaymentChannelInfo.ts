import { ethers } from "ethers";
import fs from "fs";

import { decodeEventLog, fromHex } from "viem";
import { channelFactoryAbi } from "./abi";

function extractPaymentInfo(log: any): any {
  // remove 24 bytes of padding from data

  const event = decodeEventLog({
    abi: channelFactoryAbi,
    data: log.data,
    topics: log.topics,
  });

  if (event.eventName !== "channelCreated") {
    throw new Error("Invalid event name");
    return;
  }

  // Construct payment object
  return {
    channel_id: event.args.channelId.toString(),
    address: event.args.channelAddress,
    sender: event.args.sender,
    recipient: event.args.recipient,
    duration: event.args.duration.toString(),
    tokenAddress: event.args.tokenAddress,
    balance: event.args.amount.toString(),
    nonce: 0,
    price: event.args.price.toString(),
    expiration: (event.args.timestamp + event.args.duration).toString(),
  };
}

const log = {
  address: "0xf2cabfa8b29bfb86956d1960ff748f27836e1e14",
  topics: [
    "0xa3162614b8dec8594972fac85313f8db191ab428989960edd147302037f1f2b3",
    "0x0000000000000000000000000000000000000000000000000000000000000003",
    "0x000000000000000000000000898d0dbd5850e086e6c09d2c83a26bb5f1ff8c33",
    "0x00000000000000000000000062c43323447899acb61c18181e34168903e033bf",
  ],
  data: "0x000000000000000000000000f10145a6a66b8a9004fae4c963ac36940fa7fa310000000000000000000000000000000000000000000000000000000000278d00000000000000000000000000036cbd53842c5426634e7929541ec2318f3dcf7e00000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000003e80000000000000000000000000000000000000000000000000000000067394cd4",
  blockHash:
    "0x87f0b6d188130c38d1a0841c492a8042f354cc9203be41a76e6eff69a0b097e6",
  blockNumber: "0x112f6fa",
  transactionHash:
    "0x06dc15eba8bb4e1f7614ad9b8b6ef2e3bdd412b3d83de06482ae7aa666a89fea",
  transactionIndex: "0x8",
  logIndex: "0x10",
  removed: false,
};

const paymentInfo = extractPaymentInfo(log);

fs.writeFileSync("payment.json", JSON.stringify(paymentInfo, null, 2));

console.log("Payment info saved to payment.json:", paymentInfo);
