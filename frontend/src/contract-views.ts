import { providers } from "near-api-js";

import { nearConfig } from "./near-config";
import type {
    BuyInRangeView,
    CurrentTurnView,
    GameStateView,
    PendingWithdrawal,
    TableView,
} from "./types";

const provider = new providers.JsonRpcProvider({
    url: "https://rpc.testnet.near.org",
});

async function viewFunction<T>(
    methodName: string,
    args: Record<string, unknown> = {},
): Promise<T> {
    if (!nearConfig.contractId) {
        throw new Error("Contract ID is not configured");
    }

    const result = await provider.query({
        request_type: "call_function",
        account_id: nearConfig.contractId,
        method_name: methodName,
        args_base64: Buffer.from(JSON.stringify(args)).toString("base64"),
        finality: "optimistic",
    });

    const callResult = result as unknown as { result: number[] };
    const resultString = Buffer.from(callResult.result).toString();

    return JSON.parse(resultString) as T;
}

export function getBuyInRange(): Promise<BuyInRangeView> {
    return viewFunction<BuyInRangeView>("get_buy_in_range");
}

export function getOpenTables(): Promise<TableView[]> {
    return viewFunction<TableView[]>("get_open_tables");
}

export function getTable(tableId: number): Promise<TableView | null> {
    return viewFunction<TableView | null>("get_table", {
        table_id: tableId,
    });
}

export function getGameState(tableId: number): Promise<GameStateView | null> {
    return viewFunction<GameStateView | null>("get_game_state", {
        table_id: tableId,
    });
}

export function getCurrentTurn(tableId: number): Promise<CurrentTurnView | null> {
    return viewFunction<CurrentTurnView | null>("get_current_turn", {
        table_id: tableId,
    });
}

export function getPlayerBalance(
    tableId: number,
    playerId: string,
): Promise<string | null> {
    return viewFunction<string | null>("get_player_balance", {
        table_id: tableId,
        player_id: playerId,
    });
}

export function getPendingWithdrawal(
    playerId: string,
): Promise<PendingWithdrawal | null> {
    return viewFunction<PendingWithdrawal | null>("get_pending_withdrawal", {
        player_id: playerId,
    });
}