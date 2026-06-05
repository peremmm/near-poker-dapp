import { setupWalletSelector } from "@near-wallet-selector/core";
import { setupModal } from "@near-wallet-selector/modal-ui";
import { setupMeteorWallet } from "@near-wallet-selector/meteor-wallet";

import type { WalletSelector } from "@near-wallet-selector/core";
import type { WalletSelectorModal } from "@near-wallet-selector/modal-ui";

import { nearConfig } from "./near-config";

export async function initWalletSelector(): Promise<{
    selector: WalletSelector;
    modal: WalletSelectorModal;
}> {
    const selector = await setupWalletSelector({
        network: nearConfig.networkId,
        modules: [
            setupMeteorWallet(),
        ],
    });

    const modal = setupModal(selector, {
        contractId: nearConfig.contractId,
        description: "Trustless Texas Hold'em Poker dApp on NEAR",
    });

    return { selector, modal };
}