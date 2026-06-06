export const nearConfig = {
    networkId: import.meta.env.VITE_NETWORK_ID || "testnet",
    contractId: import.meta.env.VITE_CONTRACT_ID || "",
    rpcUrl:
        import.meta.env.VITE_RPC_URL ||
        "https://test.rpc.fastnear.com",
};