import { useEffect, useState } from "react";
import type { WalletSelector } from "@near-wallet-selector/core";
import type { WalletSelectorModal } from "@near-wallet-selector/modal-ui";

import { initWalletSelector } from "./wallet";
import { nearConfig } from "./near-config";
import {
  getBuyInRange,
  getCurrentTurn,
  getGameState,
  getOpenTables,
  getPendingWithdrawal,
  getPlayerBalance,
  getTable,
} from "./contract-views";
import { formatNear, formatTimestamp } from "./format";
import type {
  BuyInRangeView,
  CurrentTurnView,
  GameStateView,
  PendingWithdrawal,
  TableView,
} from "./types";

import "@near-wallet-selector/modal-ui/styles.css";
import "./App.css";

function App() {
  const [selector, setSelector] = useState<WalletSelector | null>(null);
  const [modal, setModal] = useState<WalletSelectorModal | null>(null);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [isReady, setIsReady] = useState(false);

  const [buyInRange, setBuyInRange] = useState<BuyInRangeView | null>(null);
  const [openTables, setOpenTables] = useState<TableView[]>([]);
  const [selectedTableId, setSelectedTableId] = useState("");
  const [selectedTable, setSelectedTable] = useState<TableView | null>(null);
  const [gameState, setGameState] = useState<GameStateView | null>(null);
  const [currentTurn, setCurrentTurn] = useState<CurrentTurnView | null>(null);
  const [playerBalance, setPlayerBalance] = useState<string | null>(null);
  const [pendingWithdrawal, setPendingWithdrawal] =
      useState<PendingWithdrawal | null>(null);

  const [viewError, setViewError] = useState<string | null>(null);
  const [isLoadingViews, setIsLoadingViews] = useState(false);

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

  useEffect(() => {
    void refreshViews();
  }, [accountId]);

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

  async function refreshViews() {
    setIsLoadingViews(true);
    setViewError(null);

    try {
      const [range, tables] = await Promise.all([
        getBuyInRange(),
        getOpenTables(),
      ]);

      setBuyInRange(range);
      setOpenTables(tables);

      if (accountId) {
        const pending = await getPendingWithdrawal(accountId);
        setPendingWithdrawal(pending);
      } else {
        setPendingWithdrawal(null);
      }
    } catch (error) {
      console.error(error);
      setViewError(
          error instanceof Error ? error.message : "View call failed",
      );
    } finally {
      setIsLoadingViews(false);
    }
  }

  async function loadSelectedTable() {
    const tableId = Number(selectedTableId);

    if (!Number.isInteger(tableId) || tableId < 0) {
      setViewError("Enter a valid table ID");
      return;
    }

    setIsLoadingViews(true);
    setViewError(null);

    try {
      const [table, state, turn] = await Promise.all([
        getTable(tableId),
        getGameState(tableId),
        getCurrentTurn(tableId),
      ]);

      setSelectedTable(table);
      setGameState(state);
      setCurrentTurn(turn);

      if (accountId) {
        const balance = await getPlayerBalance(tableId, accountId);
        setPlayerBalance(balance);
      } else {
        setPlayerBalance(null);
      }
    } catch (error) {
      console.error(error);
      setViewError(
          error instanceof Error ? error.message : "Failed to load table",
      );
    } finally {
      setIsLoadingViews(false);
    }
  }

  return (
      <main className="page">
        <section className="card">
          <h1>Trustless Poker on NEAR</h1>

          <p>
            Network: <strong>{nearConfig.networkId}</strong>
          </p>

          <p>
            Contract: <strong>{nearConfig.contractId || "Not configured yet"}</strong>
          </p>

          {!isReady && <p>Loading wallet...</p>}

          {isReady && !accountId && (
              <button onClick={connectWallet}>Connect Meteor Wallet</button>
          )}

          {isReady && accountId && (
              <div className="wallet-box">
                <p>
                  Connected as: <strong>{accountId}</strong>
                </p>

                <button onClick={disconnectWallet}>Disconnect</button>
              </div>
          )}

          <hr />

          <div className="section-header">
            <h2>Contract Views</h2>
            <button onClick={refreshViews} disabled={isLoadingViews}>
              Refresh
            </button>
          </div>

          {viewError && <p className="error">{viewError}</p>}
          {isLoadingViews && <p>Loading contract views...</p>}

          <section>
            <h3>Allowed Buy-in Range</h3>

            {buyInRange ? (
                <ul>
                  <li>Min: {formatNear(buyInRange.min_buy_in)}</li>
                  <li>Max: {formatNear(buyInRange.max_buy_in)}</li>
                </ul>
            ) : (
                <p>No buy-in range loaded yet.</p>
            )}
          </section>

          <section>
            <h3>Open Tables</h3>

            {openTables.length === 0 ? (
                <p>No open tables yet.</p>
            ) : (
                <ul>
                  {openTables.map((table) => (
                      <li key={table.id}>
                        Table #{table.id} — Buy-in {formatNear(table.buy_in)} —{" "}
                        Players {table.players.length}
                      </li>
                  ))}
                </ul>
            )}
          </section>

          <section>
            <h3>Load Table</h3>

            <div className="row">
              <input
                  value={selectedTableId}
                  onChange={(event) => setSelectedTableId(event.target.value)}
                  placeholder="Table ID, e.g. 0"
              />

              <button onClick={loadSelectedTable} disabled={isLoadingViews}>
                Load
              </button>
            </div>
          </section>

          {selectedTable && (
              <section>
                <h3>Selected Table #{selectedTable.id}</h3>

                <p>Status: {selectedTable.status}</p>
                <p>Creator: {selectedTable.creator_id}</p>
                <p>Buy-in: {formatNear(selectedTable.buy_in)}</p>
                <p>Players: {selectedTable.players.join(", ")}</p>
                <p>Order locked: {selectedTable.order_locked ? "Yes" : "No"}</p>
                <p>Started: {formatTimestamp(selectedTable.started_at)}</p>
                <p>Last action: {formatTimestamp(selectedTable.last_action_at)}</p>
                <p>Pot: {formatNear(selectedTable.pot)}</p>
                <p>Remaining deck count: {selectedTable.remaining_deck_count}</p>

                <h4>Player Balances</h4>
                {selectedTable.player_balances.length === 0 ? (
                    <p>No balances yet.</p>
                ) : (
                    <ul>
                      {selectedTable.player_balances.map((balance) => (
                          <li key={balance.player_id}>
                            {balance.player_id}: {formatNear(balance.balance)}
                          </li>
                      ))}
                    </ul>
                )}
              </section>
          )}

          {gameState && (
              <section>
                <h3>Game State</h3>

                <p>Status: {gameState.status}</p>
                <p>Current player: {gameState.current_player ?? "None"}</p>
                <p>Pot: {formatNear(gameState.pot)}</p>
                <p>Community cards: {gameState.community_cards.length}</p>
              </section>
          )}

          {currentTurn && (
              <section>
                <h3>Current Turn</h3>

                <p>Index: {currentTurn.current_turn_index ?? "None"}</p>
                <p>Player: {currentTurn.current_player ?? "None"}</p>
              </section>
          )}

          {accountId && (
              <section>
                <h3>Your Contract State</h3>

                <p>
                  Selected-table balance:{" "}
                  {playerBalance ? formatNear(playerBalance) : "No balance loaded"}
                </p>

                <p>
                  Pending withdrawal:{" "}
                  {pendingWithdrawal
                      ? `${formatNear(pendingWithdrawal.amount)} from table #${pendingWithdrawal.table_id}`
                      : "None"}
                </p>
              </section>
          )}
        </section>
      </main>
  );
}

export default App;