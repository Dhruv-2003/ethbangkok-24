import {
  keccak256,
  hexlify,
  toBeArray,
  Wallet,
  Contract,
  BrowserProvider,
  parseEther,
} from "ethers";
import { AbiCoder } from "ethers";
import type {
  CreateChannelParams,
  PaymentChannelResponse,
  RequestConfig,
  SignedRequest,
} from "./types";
import type { Provider } from "ethers";
import type { Signer } from "ethers";
import { channelFactoryABI } from "./abi/channel-factory";
import "dotenv/config";

interface SDKConfig {
  privateKey: string;
  provider?: Provider;
  signer?: Signer;
}

export class PaymentChannelSDK {
  private wallet: Wallet;
  private nonceMap: Map<string, number> = new Map();
  private channelStates: Map<string, PaymentChannelResponse> = new Map();
  private provider!: Provider;
  private signer!: Signer;
  private channelFactory!: Contract;
  // private config: SDKConfig;

  // constructor(config: SDKConfig) {
  constructor() {
    // todo: note, add your private key here
    this.wallet = new Wallet(
      "6d2f70a47ddf455feb6a785b9787265f28897546bd1316224300aed627ef8cfc"
    );

    // if (!config.privateKey) {
    //   throw new Error("Private key is required");
    // }

    // this.config = config;
    // // Initialize wallet with provided private key
    // this.wallet = new Wallet(config.privateKey);

    // if (config.provider) {
    //   this.provider = config.provider;
    //   this.signer = this.wallet.connect(this.provider);
    // } else if (config.signer) {
    //   this.signer = config.signer;
    //   this.provider = this.signer.provider!;
    // }
  }

  async initialize() {
    if (!this.signer) {
      const browserProvider = this.provider as BrowserProvider;
      this.signer = await browserProvider.getSigner();
    }

    this.channelFactory = new Contract(
      "0x16b12b0002487a8FB3B3877a71Ae9258d0889E1B",
      channelFactoryABI,
      this.signer
    );
  }

  /**
   * creates a new payment channel with specified parameters
   */
  async createPaymentChannel(params: CreateChannelParams): Promise<string> {
    try {
      console.log("Creating payment channel with params:", params);

      const tx = await this.channelFactory.createChannel(
        params.recipient,
        params.duration,
        params.tokenAddress,
        parseEther(params.amount)
      );

      console.log("Transaction sent:", tx.hash);
      const receipt = await tx.wait();

      const event = receipt.logs.find(
        (log: any) => log.eventName === "channelCreated"
      );

      if (!event) {
        throw new Error("Channel creation event not found");
      }

      // get channel details from event
      const channelId = event.args.channelId.toString();
      const channelAddress = event.args.channelAddress;

      console.log("Channel created:", {
        channelId,
        channelAddress,
        sender: event.args.sender,
        recipient: event.args.recipient,
        amount: event.args.amount.toString(),
        price: event.args.price.toString(),
      });

      return channelId;
    } catch (err) {
      const error = err as Error;
      throw new Error(`Failed to create payment channel: ${error.message}`);
    }
  }

  // get current channel state
  getChannelState(channelId: string): PaymentChannelResponse | undefined {
    return this.channelStates.get(channelId);
  }

  private getNonce(channelId: string): string {
    const currentNonce = this.nonceMap.get(channelId) || 0;
    this.nonceMap.set(channelId, currentNonce + 1);
    return currentNonce.toString();
  }

  /**
   * signs a request with channel details
   */
  async signRequest(
    request: RequestConfig,
    channelId: string,
    rawBody: any
  ): Promise<SignedRequest> {
    try {
      const bodyBytes = new TextEncoder().encode(
        typeof rawBody === "string" ? rawBody : JSON.stringify(rawBody)
      );

      const message = {
        channelId,
        amount: request.amount,
        nonce: this.getNonce(channelId),
        requestData: hexlify(bodyBytes),
        timestamp: Date.now(),
      };

      console.log("\nMessage to be signed:", message);

      const abiCoder = AbiCoder.defaultAbiCoder();
      const encodedMessage = abiCoder.encode(
        ["string", "string", "string", "bytes", "uint256"],
        [
          message.channelId,
          message.amount,
          message.nonce,
          bodyBytes,
          message.timestamp,
        ]
      );

      const messageHash = keccak256(encodedMessage);
      const signature = await this.wallet.signMessage(toBeArray(messageHash));

      return {
        message,
        signature,
        timestamp: message.timestamp.toString(),
      };
    } catch (err) {
      const error = err as Error;
      throw new Error(`failed to sign request: ${error.message}`);
    }
  }

  /**
   * creates an interceptor for HTTP clients (axios, fetch)
   */
  createRequestInterceptor(channelId: string) {
    return {
      request: async (config: any) => {
        try {
          const rawBody = config.data;

          const signedRequest = await this.signRequest(
            {
              amount: config.headers["x-payment-amount"] || "0",
              data: config.data || {},
            },
            channelId,
            rawBody
          );

          config.headers = {
            ...config.headers,
            "x-signature": signedRequest.signature,
            "x-message": JSON.stringify(signedRequest.message),
            "x-timestamp": signedRequest.timestamp,
          };

          console.log("Request Headers:", config.headers);
          return config;
        } catch (err) {
          const error = err as Error;
          throw new Error(`Failed to process request: ${error.message}`);
        }
      },
    };
  }

  /**
   * creates an response interceptor and extracts payment channel state
   */
  createResponseInterceptor() {
    return {
      response: (response: any) => {
        try {
          const paymentChannelStr = response.headers["x-payment"];
          if (!paymentChannelStr) {
            console.log("\nNo payment channel data in response");
            return response;
          }

          const paymentChannel: PaymentChannelResponse =
            JSON.parse(paymentChannelStr);
          const requestMessage = JSON.parse(
            response.config.headers["x-message"]
          );
          const channelId = requestMessage.channelId;

          // update channel state
          this.channelStates.set(channelId, paymentChannel);

          // update nonce
          const nextNonce = BigInt(paymentChannel.nonce) + 1n;
          this.nonceMap.set(channelId, Number(nextNonce));

          console.log("\nPayment Channel Update:");
          console.log("Channel ID:", channelId);
          console.log("Current Nonce:", paymentChannel.nonce);
          console.log("Next Nonce:", nextNonce.toString());
          console.log("Balance:", paymentChannel.balance);
          console.log("Expiration:", paymentChannel.expiration);

          return response;
        } catch (err) {
          const error = err as Error;
          throw new Error(`Failed to process response: ${error.message}`);
        }
      },
    };
  }

  /**
   * helper method to extract channelId from event logs
   */
  private getChannelIdFromLogs(logs: any[]): string {
    // todo: add more events based on contract spec
    const event = logs.find((log) => log.eventName === "channelCreated");

    if (!event) {
      throw new Error("Channel creation event not found in logs");
    }

    return event.args.channelId.toString();
  }
}
