import { useEffect, useState } from "react";
import type { WalletSelector } from "@near-wallet-selector/core";
import type { WalletSelectorModal } from "@near-wallet-selector/modal-ui";

import { initWalletSelector } from "./wallet";
import { nearConfig } from "./near-config";

import "@near-wallet-selector/modal-ui/styles.css";
import "./App.css";

function App() {
  const [selector, setSelector] = useState<WalletSelector | null>(null);
  const [modal, setModal] = useState<WalletSelectorModal | null>(null);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    async function setup() {
      const { selector, modal } = await initWalletSelector();

      setSelector(selector);
      setModal(modal);

      const state = selector.store.getState();
      const signedInAccount = state.accounts.find((account) => account.active);

      setAccountId(signedInAccount?.accountId ?? null);
      setIsReady(true);

      const subscription = selector.store.observable.subscribe((state) => {
        const activeAccount = state.accounts.find((account) => account.active);
        setAccountId(activeAccount?.accountId ?? null);
      });

      return () => subscription.unsubscribe();
    }

    setup().catch((error) => {
      console.error("Failed to initialize wallet selector:", error);
      setIsReady(true);
    });
  }, []);

  async function connectWallet() {
    if (!modal) {
      return;
    }

    modal.show();
  }

  async function disconnectWallet() {
    if (!selector) {
      return;
    }

    const wallet = await selector.wallet();
    await wallet.signOut();
    setAccountId(null);
  }

  return (
      <main className="page">
        <section className="card">
          <h1>Trustless Poker on NEAR</h1>

          <p>
            Network: <strong>{nearConfig.networkId}</strong>
          </p>

          <p>
            Contract:{" "}
            <strong>
              {nearConfig.contractId || "Not configured yet"}
            </strong>
          </p>

          {!isReady && <p>Loading wallet...</p>}

          {isReady && !accountId && (
              <button onClick={connectWallet}>
                Connect Meteor Wallet
              </button>
          )}

          {isReady && accountId && (
              <>
                <p>
                  Connected as: <strong>{accountId}</strong>
                </p>

                <button onClick={disconnectWallet}>
                  Disconnect
                </button>
              </>
          )}
        </section>
      </main>
  );
}

export default App;