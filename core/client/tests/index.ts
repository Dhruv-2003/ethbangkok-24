import axios from "axios";
import { ClientInterceptor, CreateChannelResponse } from "../src/index";

async function testSDKInterceptors() {
  console.log("\nStarting SDK Interceptor Test...");

  try {
    const sdk = new ClientInterceptor();

    const mockCreateChannelResponse: CreateChannelResponse = {
      channelId: 1n,
      sender: "0x898d0DBd5850e086E6C09D2c83A26Bb5F1ff8C33",
      recipient: "0x62C43323447899acb61C18181e34168903E033Bf",
      channelAddress: "0x19A91e578E2a8117fE4FeF93A5cf9ac886EFDa52",
      duration: 2592000n,
      tokenAddress: "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
      amount: 1000000n,
      price: 1000n,
      timestamp: 1733936718n,
    };

    // Add mock channel state to the SDK
    sdk.addNewChannel(
      mockCreateChannelResponse.channelId.toString(),
      mockCreateChannelResponse
    );

    const axiosInstance = axios.create({
      baseURL: "http://localhost:3000",
      timeout: 5000,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
    });

    // Attach interceptors from SDK
    axiosInstance.interceptors.request.use(
      sdk.createRequestInterceptor(
        mockCreateChannelResponse.channelId.toString()
      ).request
    );

    axiosInstance.interceptors.response.use(
      sdk.createResponseInterceptor().response
    );

    console.log("\nSending request to the root route...");

    // Make a GET request to the root route
    const response = await axiosInstance.get("/", {
      validateStatus: (status) => true, // Accept any status code
    });

    console.log("\nRequest Details:");
    console.log("URL:", response.config.url);
    console.log("Method:", response.config.method);
    console.log("Headers Sent:", {
      "x-Message": response.config.headers["x-Message"],
      "x-Signature": response.config.headers["x-Signature"],
      "x-Timestamp": response.config.headers["x-Timestamp"],
      "x-Payment": response.config.headers["x-Payment"],
    });

    console.log("\nResponse Details:");
    console.log("Status:", response.status);
    console.log("Data:", response.data);

    if (response.headers["x-payment"]) {
      console.log("\nUpdated Channel State from Response:");
      const updatedChannel = JSON.parse(response.headers["x-payment"]);
      console.log("New Balance:", updatedChannel.balance);
      console.log("New Nonce:", updatedChannel.nonce);
    }

    const finalState = sdk.getChannelState(
      mockCreateChannelResponse.channelId.toString()
    );
    console.log("\nFinal SDK Channel State:", finalState);

    // Make another GET request to the root route for checking if nonce and balance works
    const response2 = await axiosInstance.get("/", {
      validateStatus: (status) => true, // Accept any status code
    });
  } catch (error) {
    if (axios.isAxiosError(error)) {
      console.error("\nRequest Failed:");
      console.log("Status:", error.response?.status);
      console.log("Headers:", error.response?.headers);
      console.log("Data:", error.response?.data);
      if (error.response?.headers["x-payment"]) {
        console.log("\nChannel State in Error Response:");
        console.log(error.response.headers["x-payment"]);
      }
    } else {
      console.error("\nUnexpected Error:", error);
    }
  }
}

// Run test
console.log("=== Payment Channel SDK Interceptor Test ===");
testSDKInterceptors()
  .then(() => console.log("\nTest completed"))
  .catch(console.error);
