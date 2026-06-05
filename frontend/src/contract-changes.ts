import { actionCreators } from "@near-wallet-selector/core";
import type { WalletSelector } from "@near-wallet-selector/core";

import { nearConfig } from "./near-config";

const ONE_NEAR = 1_000_000_000_000_000_000_000_000n;
const TGAS = 1_000_000_000_000n;

export function nearToYocto(amount: string): string {
    const trimmed = amount.trim();

    if (!trimmed) {
        return "0";
    }

    const [wholePart, fractionalPart = ""] = trimmed.split(".");

    const whole = BigInt(wholePart || "0") * ONE_NEAR;
    const fractional = BigInt(
        fractionalPart.padEnd(24, "0").slice(0, 24) || "0",
    );

    return (whole + fractional).toString();
}

export async function callChangeMethod(
    selector: WalletSelector,
    methodName: string,
    args: Record<string, unknown>,
    depositYocto = "0",
) {
    if (!nearConfig.contractId) {
        throw new Error("Contract ID is not configured");
    }

    const wallet = await selector.wallet();

    return wallet.signAndSendTransaction({
        receiverId: nearConfig.contractId,
        actions: [
            actionCreators.functionCall(
                methodName,
                args,
                100n * TGAS,
                BigInt(depositYocto),
            ),
        ],
    });
}