export type Balance = string;

export type BuyInRangeView = {
    min_buy_in: Balance;
    max_buy_in: Balance;
};

export type TableStatus =
    | "WaitingForPlayers"
    | "Active"
    | "Finished"
    | "Cancelled";

export type Card = {
    rank: string;
    suit: string;
};

export type PlayerCards = {
    player_id: string;
    cards: Card[];
};

export type PlayerBalance = {
    player_id: string;
    balance: Balance;
};

export type PlayerAction =
    | "Check"
    | "Call"
    | { Raise: { amount: Balance } }
    | "Fold";

export type ActionRecord = {
    player_id: string;
    action: PlayerAction;
    timestamp: number;
};

export type RoundResult = {
    winner_id: string;
    pot_awarded: Balance;
    resolved_at: number;
};

export type PendingWithdrawal = {
    table_id: number;
    player_id: string;
    amount: Balance;
    requested_at: number;
};

export type TableView = {
    id: number;
    creator_id: string;
    buy_in: Balance;
    players: string[];
    status: TableStatus;
    created_at: number;
    order_locked: boolean;
    current_turn_index: number | null;
    started_at: number | null;
    last_action_at: number | null;
    player_cards: PlayerCards[];
    community_cards: Card[];
    remaining_deck_count: number;
    action_history: ActionRecord[];
    pot: Balance;
    player_balances: PlayerBalance[];
    round_result: RoundResult | null;
};

export type CurrentTurnView = {
    table_id: number;
    current_turn_index: number | null;
    current_player: string | null;
};

export type GameStateView = {
    table_id: number;
    status: TableStatus;
    players: string[];
    current_turn_index: number | null;
    current_player: string | null;
    pot: Balance;
    community_cards: Card[];
    remaining_deck_count: number;
    round_result: RoundResult | null;
    last_action_at: number | null;
};