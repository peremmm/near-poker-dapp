use near_sdk::{
    borsh, env, ext_contract, near, AccountId, BorshStorageKey, Gas, NearToken,
    PanicOnDefault, Promise, PromiseResult,
};
use near_sdk::json_types::U128;
use near_sdk::store::UnorderedMap;
use std::collections::HashSet;

const TABLE_STORAGE_OVERHEAD_BYTES: u64 = 256;
const MAX_PLAYERS: usize = 2;
const WITHDRAW_CALLBACK_GAS: Gas = Gas::from_tgas(10);
// 10 minutes
const ABANDON_TIMEOUT_NS: u64 = 10 * 60 * 1_000_000_000;
const SMALL_BLIND: Balance = 100_000_000_000_000_000_000_000; // 0.1 NEAR
const BIG_BLIND: Balance = 200_000_000_000_000_000_000_000;   // 0.2 NEAR

pub type Balance = u128;

#[ext_contract(ext_self)]
trait ExtSelf {
    fn on_withdraw_complete(
        &mut self,
        player_id: AccountId,
        table_id: u64,
        amount: Balance,
    ) -> bool;
}

#[derive(BorshStorageKey)]
#[near(serializers = [borsh])]
pub enum StorageKey {
    Tables,
    TablesV2,
    TablesV3,
    PendingWithdrawals,
}

#[near(serializers = [json])]
pub struct BuyInRangeView {
    pub min_buy_in: Balance,
    pub max_buy_in: Balance,
}

#[derive(Clone, PartialEq, Eq, Debug)]
#[near(serializers = [borsh, json])]
pub enum TableStatus {
    WaitingForPlayers,
    Active,
    Finished,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub enum GameStage {
    Waiting,
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[near(serializers = [borsh, json])]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[near(serializers = [borsh, json])]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[near(serializers = [borsh, json])]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct PlayerCards {
    pub player_id: AccountId,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct PlayerBalance {
    pub player_id: AccountId,
    pub balance: Balance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub enum PlayerAction {
    Check,
    Call,
    Raise { amount: U128 },
    Fold,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct ActionRecord {
    pub player_id: AccountId,
    pub action: PlayerAction,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct RoundResult {
    pub winner_id: AccountId,
    pub pot_awarded: Balance,
    pub resolved_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PokerHandScore {
    pub category: u8,
    pub kickers: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct PendingWithdrawal {
    pub table_id: u64,
    pub player_id: AccountId,
    pub amount: Balance,
    pub requested_at: u64,
}

#[derive(Clone)]
#[near(serializers = [borsh])]
pub struct Table {
    pub id: u64,
    pub creator_id: AccountId,
    pub buy_in: Balance,
    pub players: Vec<AccountId>,
    pub status: TableStatus,
    pub game_stage: GameStage,
    pub created_at: u64,
    pub order_locked: bool,
    pub current_turn_index: Option<u8>,
    pub started_at: Option<u64>,
    pub last_action_at: Option<u64>,
    pub deck: Vec<Card>,
    pub player_cards: Vec<PlayerCards>,
    pub community_cards: Vec<Card>,
    pub action_history: Vec<ActionRecord>,
    pub pot: Balance,
    pub player_balances: Vec<PlayerBalance>,
    pub small_blind: Balance,
    pub big_blind: Balance,
    pub small_blind_index: Option<u8>,
    pub big_blind_index: Option<u8>,
    pub round_result: Option<RoundResult>,
}

#[near(serializers = [json])]
pub struct TableView {
    pub id: u64,
    pub creator_id: AccountId,
    pub buy_in: Balance,
    pub players: Vec<AccountId>,
    pub status: TableStatus,
    pub game_stage: GameStage,
    pub created_at: u64,
    pub order_locked: bool,
    pub current_turn_index: Option<u8>,
    pub started_at: Option<u64>,
    pub last_action_at: Option<u64>,
    pub player_cards: Vec<PlayerCards>,
    pub community_cards: Vec<Card>,
    pub remaining_deck_count: usize,
    pub action_history: Vec<ActionRecord>,
    pub pot: Balance,
    pub player_balances: Vec<PlayerBalance>,
    pub small_blind: Balance,
    pub big_blind: Balance,
    pub small_blind_index: Option<u8>,
    pub big_blind_index: Option<u8>,
    pub round_result: Option<RoundResult>,
}

#[near(serializers = [json])]
pub struct GameStateView {
    pub table_id: u64,
    pub status: TableStatus,
    pub game_stage: GameStage,
    pub players: Vec<AccountId>,
    pub current_turn_index: Option<u8>,
    pub current_player: Option<AccountId>,
    pub pot: Balance,
    pub community_cards: Vec<Card>,
    pub remaining_deck_count: usize,
    pub round_result: Option<RoundResult>,
    pub last_action_at: Option<u64>,
}

#[near(serializers = [json])]
pub struct CurrentTurnView {
    pub table_id: u64,
    pub current_turn_index: Option<u8>,
    pub current_player: Option<AccountId>,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    min_buy_in: Balance,
    max_buy_in: Balance,
    paused: bool,
    tables: UnorderedMap<u64, Table>,
    next_table_id: u64,
    pending_withdrawals: UnorderedMap<AccountId, PendingWithdrawal>,
}

#[near]
impl Contract {
    #[init]
    #[init]
    pub fn new(owner_id: AccountId, min_buy_in: U128, max_buy_in: U128) -> Self {
        let min_buy_in: Balance = min_buy_in.0;
        let max_buy_in: Balance = max_buy_in.0;
        assert!(!env::state_exists(), "Contract is already initialized");
        assert!(min_buy_in > 0, "Minimum buy-in must be greater than zero");
        assert!(
            min_buy_in <= max_buy_in,
            "Minimum buy-in must be less than or equal to maximum buy-in"
        );

        Self {
            owner_id,
            min_buy_in,
            max_buy_in,
            paused: false,
            tables: UnorderedMap::new(StorageKey::Tables),
            next_table_id: 0,
            pending_withdrawals: UnorderedMap::new(StorageKey::PendingWithdrawals),
        }
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn get_buy_in_range(&self) -> BuyInRangeView {
        BuyInRangeView {
            min_buy_in: self.min_buy_in,
            max_buy_in: self.max_buy_in,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_buy_in_range(&mut self, min_buy_in: U128, max_buy_in: U128) {
        let min_buy_in: Balance = min_buy_in.0;
        let max_buy_in: Balance = max_buy_in.0;
        self.assert_owner();

        assert!(min_buy_in > 0, "Minimum buy-in must be greater than zero");
        assert!(
            min_buy_in <= max_buy_in,
            "Minimum buy-in must be less than or equal to maximum buy-in"
        );

        self.min_buy_in = min_buy_in;
        self.max_buy_in = max_buy_in;
    }

    pub fn dev_reset_tables(&mut self) {
        self.assert_owner();

        self.tables = UnorderedMap::new(StorageKey::TablesV3);
        self.next_table_id = 0;
    }

    pub fn pause(&mut self) {
        self.assert_owner();

        self.paused = true;
    }

    pub fn unpause(&mut self) {
        self.assert_owner();

        self.paused = false;
    }

    #[payable]
    pub fn create_table(&mut self, buy_in: U128) -> u64 {
        let buy_in: Balance = buy_in.0;

        self.assert_not_paused();

        assert!(
            buy_in >= self.min_buy_in,
            "Buy-in is below the minimum allowed"
        );

        assert!(
            buy_in <= self.max_buy_in,
            "Buy-in is above the maximum allowed"
        );

        let attached_deposit = env::attached_deposit().as_yoctonear();
        let initial_storage = env::storage_usage();

        let creator_id = env::predecessor_account_id();
        let table_id = self.next_table_id;

        let table = Table {
            id: table_id,
            creator_id: creator_id.clone(),
            buy_in,
            players: vec![creator_id.clone()],
            status: TableStatus::WaitingForPlayers,
            game_stage: GameStage::Waiting,
            created_at: env::block_timestamp(),
            order_locked: false,
            current_turn_index: None,
            started_at: None,
            last_action_at: None,
            deck: Vec::new(),
            player_cards: Vec::new(),
            community_cards: Vec::new(),
            action_history: Vec::new(),
            pot: 0,
            player_balances: Vec::new(),
            small_blind: SMALL_BLIND,
            big_blind: BIG_BLIND,
            small_blind_index: None,
            big_blind_index: None,
            round_result: None,
        };

        let estimated_table_bytes =
            borsh::to_vec(&table).expect("Failed to serialize table").len() as u64
                + TABLE_STORAGE_OVERHEAD_BYTES;

        let estimated_storage_cost =
            Balance::from(estimated_table_bytes) * env::storage_byte_cost().as_yoctonear();

        self.tables.insert(table_id, table);
        self.next_table_id += 1;

        let final_storage = env::storage_usage();
        let storage_used = final_storage.saturating_sub(initial_storage);
        let measured_storage_cost =
            Balance::from(storage_used) * env::storage_byte_cost().as_yoctonear();

        let storage_cost = measured_storage_cost.max(estimated_storage_cost);
        let required_deposit = buy_in + storage_cost;

        assert!(
            attached_deposit >= required_deposit,
            "Insufficient deposit for buy-in and storage"
        );

        let refund = attached_deposit - required_deposit;

        if refund > 0 {
            Promise::new(creator_id).transfer(NearToken::from_yoctonear(refund));
        }

        table_id
    }

    #[payable]
    pub fn join_table(&mut self, table_id: u64) {
        self.assert_not_paused();

        let attached_deposit = env::attached_deposit().as_yoctonear();
        let joiner_id = env::predecessor_account_id();

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert_eq!(
            table.status,
            TableStatus::WaitingForPlayers,
            "Table is not waiting for players"
        );

        assert!(
            !table.players.contains(&joiner_id),
            "Player already joined this table"
        );

        assert!(
            table.players.len() < MAX_PLAYERS,
            "Table is already full"
        );

        assert!(
            attached_deposit > table.buy_in,
            "Attach buy-in plus storage deposit"
        );

        let initial_storage = env::storage_usage();

        table.players.push(joiner_id.clone());

        if table.players.len() == MAX_PLAYERS {
            self.start_game(&mut table);
        }

        let estimated_table_bytes =
            borsh::to_vec(&table).expect("Failed to serialize table").len() as u64
                + TABLE_STORAGE_OVERHEAD_BYTES;

        let estimated_storage_cost =
            Balance::from(estimated_table_bytes) * env::storage_byte_cost().as_yoctonear();

        self.tables.insert(table_id, table);

        let final_storage = env::storage_usage();
        let storage_used = final_storage.saturating_sub(initial_storage);

        let measured_storage_cost =
            Balance::from(storage_used) * env::storage_byte_cost().as_yoctonear();

        let storage_cost = measured_storage_cost.max(estimated_storage_cost);

        let required_deposit = table_buy_in_plus_storage(table_id, self, storage_cost);

        assert!(
            attached_deposit >= required_deposit,
            "Insufficient deposit for buy-in and storage"
        );

        let refund = attached_deposit - required_deposit;

        if refund > 0 {
            Promise::new(joiner_id).transfer(NearToken::from_yoctonear(refund));
        }
    }

    pub fn submit_action(&mut self, table_id: u64, action: PlayerAction) {
        self.assert_not_paused();

        let actor_id = env::predecessor_account_id();

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert_eq!(
            table.status,
            TableStatus::Active,
            "Table is not active"
        );

        assert!(
            table.players.contains(&actor_id),
            "Only table players can submit actions"
        );

        let current_turn_index = table
            .current_turn_index
            .expect("Current turn is not set") as usize;

        let current_player = table
            .players
            .get(current_turn_index)
            .expect("Current turn player does not exist");

        assert_eq!(
            current_player,
            &actor_id,
            "Only the current player can act"
        );

        let is_fold = matches!(action, PlayerAction::Fold);

        match &action {
            PlayerAction::Raise { amount } => {
                let amount: Balance = amount.0;

                assert!(amount > 0, "Raise amount must be greater than zero");

                let player_balance = table
                    .player_balances
                    .iter_mut()
                    .find(|balance| balance.player_id == actor_id)
                    .expect("Player balance does not exist");

                assert!(
                    player_balance.balance >= amount,
                    "Raise amount exceeds player balance"
                );

                player_balance.balance -= amount;
                table.pot += amount;
            }
            PlayerAction::Check | PlayerAction::Call | PlayerAction::Fold => {}
        }

        table.action_history.push(ActionRecord {
            player_id: actor_id.clone(),
            action,
            timestamp: env::block_timestamp(),
        });

        if is_fold {
            self.resolve_fold(&mut table, actor_id);
        } else {
            table.current_turn_index =
                Some(((current_turn_index + 1) % table.players.len()) as u8);
        }

        table.last_action_at = Some(env::block_timestamp());

        self.tables.insert(table_id, table);
    }

    pub fn advance_stage(&mut self, table_id: u64) {
        self.assert_not_paused();

        let caller_id = env::predecessor_account_id();

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert_eq!(
            table.status,
            TableStatus::Active,
            "Table is not active"
        );

        assert!(
            table.players.contains(&caller_id) || caller_id == self.owner_id,
            "Only table players or owner can advance stage"
        );

        match table.game_stage {
            GameStage::PreFlop => {
                Self::deal_community_cards(&mut table, 3);
                table.game_stage = GameStage::Flop;
            }
            GameStage::Flop => {
                Self::deal_community_cards(&mut table, 1);
                table.game_stage = GameStage::Turn;
            }
            GameStage::Turn => {
                Self::deal_community_cards(&mut table, 1);
                table.game_stage = GameStage::River;
            }
            GameStage::River => {
                table.game_stage = GameStage::Showdown;
            }
            GameStage::Showdown => {
                env::panic_str("Game is already at showdown");
            }
            GameStage::Waiting => {
                env::panic_str("Game has not started");
            }
        }

        table.last_action_at = Some(env::block_timestamp());

        self.tables.insert(table_id, table);
    }

    pub fn resolve_round(&mut self, table_id: u64, winner_id: AccountId) {
        self.assert_not_paused();
        self.assert_owner();

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert_eq!(
            table.status,
            TableStatus::Active,
            "Table is not active"
        );

        assert!(
            table.players.contains(&winner_id),
            "Winner must be a table player"
        );

        assert!(
            table.round_result.is_none(),
            "Round already resolved"
        );

        assert!(
            table.pot > 0,
            "Cannot resolve round with empty pot"
        );

        let pot_awarded = table.pot;

        let winner_balance = table
            .player_balances
            .iter_mut()
            .find(|balance| balance.player_id == winner_id)
            .expect("Winner balance does not exist");

        winner_balance.balance += pot_awarded;

        table.pot = 0;
        table.status = TableStatus::Finished;
        table.current_turn_index = None;
        table.round_result = Some(RoundResult {
            winner_id,
            pot_awarded,
            resolved_at: env::block_timestamp(),
        });

        self.tables.insert(table_id, table);
    }

    pub fn resolve_round_by_evaluation(&mut self, table_id: u64) {
        self.assert_not_paused();

        let caller_id = env::predecessor_account_id();

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert_eq!(
            table.status,
            TableStatus::Active,
            "Table is not active"
        );

        assert_eq!(
            table.game_stage,
            GameStage::Showdown,
            "Round can only be evaluated at showdown"
        );

        assert_eq!(
            table.community_cards.len(),
            5,
            "Evaluation requires five community cards"
        );

        assert!(
            table.players.contains(&caller_id) || caller_id == self.owner_id,
            "Only table players or owner can resolve by evaluation"
        );

        assert_eq!(
            table.players.len(),
            2,
            "Evaluation currently supports 2 players only"
        );

        let player_one = table.players[0].clone();
        let player_two = table.players[1].clone();

        let mut player_one_cards = Self::player_cards_for(&table, &player_one);
        player_one_cards.extend(table.community_cards.clone());

        let mut player_two_cards = Self::player_cards_for(&table, &player_two);
        player_two_cards.extend(table.community_cards.clone());

        let player_one_score = Self::best_hand_score(&player_one_cards);
        let player_two_score = Self::best_hand_score(&player_two_cards);

        if player_one_score > player_two_score {
            self.award_pot_to_winner(&mut table, player_one);
        } else if player_two_score > player_one_score {
            self.award_pot_to_winner(&mut table, player_two);
        } else {
            self.split_pot_between_players(&mut table);
        }

        self.tables.insert(table_id, table);
    }

    fn resolve_fold(
        &self,
        table: &mut Table,
        folding_player_id: AccountId,
    ) {
        assert_eq!(
            table.players.len(),
            2,
            "Fold resolution currently supports 2 players only"
        );

        let winner_id = table
            .players
            .iter()
            .find(|player_id| **player_id != folding_player_id)
            .expect("Opponent should exist")
            .clone();

        let pot_awarded = table.pot;

        let winner_balance = table
            .player_balances
            .iter_mut()
            .find(|balance| balance.player_id == winner_id)
            .expect("Winner balance does not exist");

        winner_balance.balance += pot_awarded;

        table.pot = 0;
        table.status = TableStatus::Finished;
        table.current_turn_index = None;
        table.round_result = Some(RoundResult {
            winner_id,
            pot_awarded,
            resolved_at: env::block_timestamp(),
        });
    }

    fn award_pot_to_winner(
        &self,
        table: &mut Table,
        winner_id: AccountId,
    ) {
        let pot_awarded = table.pot;

        let winner_balance = table
            .player_balances
            .iter_mut()
            .find(|balance| balance.player_id == winner_id)
            .expect("Winner balance does not exist");

        winner_balance.balance += pot_awarded;

        table.pot = 0;
        table.status = TableStatus::Finished;
        table.current_turn_index = None;
        table.round_result = Some(RoundResult {
            winner_id,
            pot_awarded,
            resolved_at: env::block_timestamp(),
        });
    }

    fn split_pot_between_players(&self, table: &mut Table) {
        assert_eq!(
            table.players.len(),
            2,
            "Split pot currently supports 2 players only"
        );

        let pot_awarded = table.pot;
        let share = pot_awarded / 2;
        let remainder = pot_awarded % 2;

        for player_id in table.players.iter() {
            let balance = table
                .player_balances
                .iter_mut()
                .find(|balance| balance.player_id == *player_id)
                .expect("Player balance does not exist");

            balance.balance += share;
        }

        if remainder > 0 {
            let first_player = table.players[0].clone();

            let first_balance = table
                .player_balances
                .iter_mut()
                .find(|balance| balance.player_id == first_player)
                .expect("First player balance does not exist");

            first_balance.balance += remainder;
        }

        table.pot = 0;
        table.status = TableStatus::Finished;
        table.current_turn_index = None;
        table.round_result = Some(RoundResult {
            winner_id: "split-pot.testnet".parse().expect("Valid split pot account"),
            pot_awarded,
            resolved_at: env::block_timestamp(),
        });
    }

    fn player_cards_for(
        table: &Table,
        player_id: &AccountId,
    ) -> Vec<Card> {
        table
            .player_cards
            .iter()
            .find(|hand| &hand.player_id == player_id)
            .expect("Player cards do not exist")
            .cards
            .clone()
    }

    pub fn withdraw(&mut self, table_id: u64) -> Promise {
        self.assert_not_paused();

        let player_id = env::predecessor_account_id();

        assert!(
            self.pending_withdrawals.get(&player_id).is_none(),
            "Player already has a pending withdrawal"
        );

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert!(
            table.status == TableStatus::Finished || table.status == TableStatus::Cancelled,
            "Withdrawals are only allowed after the table is finished or cancelled"
        );

        assert!(
            table.players.contains(&player_id),
            "Only table players can withdraw"
        );

        let player_balance = table
            .player_balances
            .iter_mut()
            .find(|balance| balance.player_id == player_id)
            .expect("Player balance does not exist");

        let amount = player_balance.balance;

        assert!(amount > 0, "No balance available to withdraw");

        player_balance.balance = 0;

        self.tables.insert(table_id, table);

        self.pending_withdrawals.insert(
            player_id.clone(),
            PendingWithdrawal {
                table_id,
                player_id: player_id.clone(),
                amount,
                requested_at: env::block_timestamp(),
            },
        );

        Promise::new(player_id.clone())
            .transfer(NearToken::from_yoctonear(amount))
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(WITHDRAW_CALLBACK_GAS)
                    .on_withdraw_complete(player_id, table_id, amount),
            )
    }

    #[private]
    pub fn on_withdraw_complete(
        &mut self,
        player_id: AccountId,
        table_id: u64,
        amount: Balance,
    ) -> bool {
        let pending = self
            .pending_withdrawals
            .get(&player_id)
            .expect("Pending withdrawal does not exist")
            .clone();

        assert_eq!(
            pending.table_id, table_id,
            "Pending withdrawal table mismatch"
        );

        assert_eq!(
            pending.amount, amount,
            "Pending withdrawal amount mismatch"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                self.pending_withdrawals.remove(&player_id);
                true
            }
            PromiseResult::Failed => {
                let mut table = self
                    .tables
                    .get(&table_id)
                    .expect("Table does not exist")
                    .clone();

                let player_balance = table
                    .player_balances
                    .iter_mut()
                    .find(|balance| balance.player_id == player_id)
                    .expect("Player balance does not exist");

                player_balance.balance += amount;

                self.tables.insert(table_id, table);
                self.pending_withdrawals.remove(&player_id);

                false
            }
        }
    }

    pub fn claim_timeout_refund(&mut self, table_id: u64) {
        self.assert_not_paused();

        let caller_id = env::predecessor_account_id();

        let mut table = self
            .tables
            .get(&table_id)
            .expect("Table does not exist")
            .clone();

        assert_eq!(
            table.status,
            TableStatus::Active,
            "Timeout refund is only available for active tables"
        );

        assert!(
            table.players.contains(&caller_id),
            "Only table players can claim timeout refund"
        );

        let last_action_at = table
            .last_action_at
            .expect("Last action timestamp is not set");

        assert!(
            env::block_timestamp() >= last_action_at + ABANDON_TIMEOUT_NS,
            "Timeout has not passed yet"
        );

        if table.pot > 0 {
            let player_count = table.players.len() as u128;
            let refund_share = table.pot / player_count;
            let remainder = table.pot % player_count;

            for player_balance in table.player_balances.iter_mut() {
                player_balance.balance += refund_share;
            }

            if remainder > 0 {
                let first_player = table
                    .players
                    .first()
                    .expect("Table should have at least one player")
                    .clone();

                let first_player_balance = table
                    .player_balances
                    .iter_mut()
                    .find(|balance| balance.player_id == first_player)
                    .expect("First player balance does not exist");

                first_player_balance.balance += remainder;
            }

            table.pot = 0;
        }

        table.status = TableStatus::Cancelled;
        table.current_turn_index = None;

        self.tables.insert(table_id, table);
    }

    pub fn get_pending_withdrawal(&self, player_id: AccountId) -> Option<PendingWithdrawal> {
        self.pending_withdrawals.get(&player_id).cloned()
    }

    pub fn get_table(&self, table_id: u64) -> Option<TableView> {
        self.tables.get(&table_id).map(|table| TableView {
            id: table.id,
            creator_id: table.creator_id.clone(),
            buy_in: table.buy_in,
            players: table.players.clone(),
            status: table.status.clone(),
            game_stage: table.game_stage.clone(),
            created_at: table.created_at,
            order_locked: table.order_locked,
            current_turn_index: table.current_turn_index,
            started_at: table.started_at,
            last_action_at: table.last_action_at,
            player_cards: table.player_cards.clone(),
            community_cards: table.community_cards.clone(),
            remaining_deck_count: table.deck.len(),
            action_history: table.action_history.clone(),
            pot: table.pot,
            player_balances: table.player_balances.clone(),
            small_blind: table.small_blind,
            big_blind: table.big_blind,
            small_blind_index: table.small_blind_index,
            big_blind_index: table.big_blind_index,
            round_result: table.round_result.clone(),
        })
    }

    pub fn get_open_tables(&self) -> Vec<TableView> {
        self.tables
            .iter()
            .filter(|(_, table)| table.status == TableStatus::WaitingForPlayers)
            .map(|(_, table)| TableView {
                id: table.id,
                creator_id: table.creator_id.clone(),
                buy_in: table.buy_in,
                players: table.players.clone(),
                status: table.status.clone(),
                game_stage: table.game_stage.clone(),
                created_at: table.created_at,
                order_locked: table.order_locked,
                current_turn_index: table.current_turn_index,
                started_at: table.started_at,
                last_action_at: table.last_action_at,
                player_cards: table.player_cards.clone(),
                community_cards: table.community_cards.clone(),
                remaining_deck_count: table.deck.len(),
                action_history: table.action_history.clone(),
                pot: table.pot,
                player_balances: table.player_balances.clone(),
                small_blind: table.small_blind,
                big_blind: table.big_blind,
                small_blind_index: table.small_blind_index,
                big_blind_index: table.big_blind_index,
                round_result: table.round_result.clone(),
            })
            .collect()
    }

    pub fn get_player_balance(
        &self,
        table_id: u64,
        player_id: AccountId,
    ) -> Option<Balance> {
        let table = self.tables.get(&table_id)?;

        table
            .player_balances
            .iter()
            .find(|balance| balance.player_id == player_id)
            .map(|balance| balance.balance)
    }

    pub fn get_current_turn(&self, table_id: u64) -> Option<CurrentTurnView> {
        let table = self.tables.get(&table_id)?;

        let current_player = table.current_turn_index.and_then(|index| {
            table.players.get(index as usize).cloned()
        });

        Some(CurrentTurnView {
            table_id,
            current_turn_index: table.current_turn_index,
            current_player,
        })
    }

    pub fn get_game_state(&self, table_id: u64) -> Option<GameStateView> {
        let table = self.tables.get(&table_id)?;

        let current_player = table.current_turn_index.and_then(|index| {
            table.players.get(index as usize).cloned()
        });

        Some(GameStateView {
            table_id,
            status: table.status.clone(),
            game_stage: table.game_stage.clone(),
            players: table.players.clone(),
            current_turn_index: table.current_turn_index,
            current_player,
            pot: table.pot,
            community_cards: table.community_cards.clone(),
            remaining_deck_count: table.deck.len(),
            round_result: table.round_result.clone(),
            last_action_at: table.last_action_at,
        })
    }

    fn start_game(&self, table: &mut Table) {
        assert_eq!(
            table.status,
            TableStatus::WaitingForPlayers,
            "Table is not waiting for players"
        );

        assert_eq!(
            table.players.len(),
            MAX_PLAYERS,
            "Not enough players to start game"
        );

        let mut deck = Self::build_deck();
        Self::shuffle_deck(&mut deck);

        let mut player_cards = Vec::new();
        let mut player_balances = Vec::new();

        for player_id in table.players.iter() {
            let first_card = deck.pop().expect("Deck should contain enough cards");
            let second_card = deck.pop().expect("Deck should contain enough cards");

            player_cards.push(PlayerCards {
                player_id: player_id.clone(),
                cards: vec![first_card, second_card],
            });

            player_balances.push(PlayerBalance {
                player_id: player_id.clone(),
                balance: table.buy_in,
            });
        }

        assert!(
            table.buy_in >= BIG_BLIND,
            "Buy-in must be at least the big blind"
        );

        let small_blind_index: usize = 0;
        let big_blind_index: usize = 1;

        player_balances[small_blind_index].balance -= SMALL_BLIND;
        player_balances[big_blind_index].balance -= BIG_BLIND;

        let starting_pot = SMALL_BLIND + BIG_BLIND;

        let now = env::block_timestamp();

        table.status = TableStatus::Active;
        table.game_stage = GameStage::PreFlop;
        table.order_locked = true;
        table.current_turn_index = Some(0);
        table.started_at = Some(now);
        table.last_action_at = Some(now);
        table.deck = deck;
        table.player_cards = player_cards;
        table.community_cards = Vec::new();
        table.pot = starting_pot;
        table.player_balances = player_balances;
        table.small_blind = SMALL_BLIND;
        table.big_blind = BIG_BLIND;
        table.small_blind_index = Some(small_blind_index as u8);
        table.big_blind_index = Some(big_blind_index as u8);
    }

    fn build_deck() -> Vec<Card> {
        let suits = vec![
            Suit::Clubs,
            Suit::Diamonds,
            Suit::Hearts,
            Suit::Spades,
        ];

        let ranks = vec![
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];

        let mut deck = Vec::new();

        for suit in suits {
            for rank in ranks.iter() {
                deck.push(Card {
                    rank: rank.clone(),
                    suit: suit.clone(),
                });
            }
        }

        deck
    }

    fn shuffle_deck(deck: &mut Vec<Card>) {
        let seed = env::random_seed();
        let seed_bytes: &[u8] = seed.as_ref();

        let mut state = env::block_timestamp();

        for byte in seed_bytes.iter() {
            state = state
                .wrapping_mul(31)
                .wrapping_add(u64::from(*byte));
        }

        for i in (1..deck.len()).rev() {
            let seed_byte = u64::from(seed_bytes[i % seed_bytes.len()]);

            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407)
                .wrapping_add(seed_byte);

            let j = (state as usize) % (i + 1);

            deck.swap(i, j);
        }
    }

    fn deal_community_cards(table: &mut Table, count: usize) {
        assert!(
            table.deck.len() >= count,
            "Deck does not contain enough cards"
        );

        for _ in 0..count {
            let card = table.deck.pop().expect("Deck should contain enough cards");
            table.community_cards.push(card);
        }
    }

    fn rank_value(rank: &Rank) -> u8 {
        match rank {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }

    fn best_hand_score(cards: &[Card]) -> PokerHandScore {
        assert_eq!(
            cards.len(),
            7,
            "Best hand evaluation requires exactly 7 cards"
        );

        let mut best_score: Option<PokerHandScore> = None;

        for a in 0..cards.len() - 4 {
            for b in a + 1..cards.len() - 3 {
                for c in b + 1..cards.len() - 2 {
                    for d in c + 1..cards.len() - 1 {
                        for e in d + 1..cards.len() {
                            let hand = vec![
                                cards[a].clone(),
                                cards[b].clone(),
                                cards[c].clone(),
                                cards[d].clone(),
                                cards[e].clone(),
                            ];

                            let score = Self::score_five_card_hand(&hand);

                            if best_score
                                .as_ref()
                                .map(|current| score > *current)
                                .unwrap_or(true)
                            {
                                best_score = Some(score);
                            }
                        }
                    }
                }
            }
        }

        best_score.expect("At least one 5-card hand should exist")
    }

    fn score_five_card_hand(cards: &[Card]) -> PokerHandScore {
        assert_eq!(
            cards.len(),
            5,
            "Five-card hand scoring requires exactly 5 cards"
        );

        let mut ranks: Vec<u8> = cards
            .iter()
            .map(|card| Self::rank_value(&card.rank))
            .collect();

        ranks.sort_by(|a, b| b.cmp(a));

        let is_flush = cards
            .iter()
            .all(|card| card.suit == cards[0].suit);

        let straight_high = Self::straight_high_card(&ranks);

        let mut rank_counts: Vec<(u8, usize)> = Vec::new();

        for rank in ranks.iter() {
            if let Some((_, count)) = rank_counts
                .iter_mut()
                .find(|(existing_rank, _)| existing_rank == rank)
            {
                *count += 1;
            } else {
                rank_counts.push((*rank, 1));
            }
        }

        rank_counts.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.0.cmp(&a.0))
        });

        if is_flush {
            if let Some(high) = straight_high {
                return PokerHandScore {
                    category: 8,
                    kickers: vec![high],
                };
            }
        }

        if rank_counts[0].1 == 4 {
            let four_rank = rank_counts[0].0;
            let kicker = rank_counts
                .iter()
                .find(|(_, count)| *count == 1)
                .map(|(rank, _)| *rank)
                .expect("Four of a kind should have kicker");

            return PokerHandScore {
                category: 7,
                kickers: vec![four_rank, kicker],
            };
        }

        if rank_counts[0].1 == 3 && rank_counts[1].1 == 2 {
            return PokerHandScore {
                category: 6,
                kickers: vec![rank_counts[0].0, rank_counts[1].0],
            };
        }

        if is_flush {
            return PokerHandScore {
                category: 5,
                kickers: ranks,
            };
        }

        if let Some(high) = straight_high {
            return PokerHandScore {
                category: 4,
                kickers: vec![high],
            };
        }

        if rank_counts[0].1 == 3 {
            let three_rank = rank_counts[0].0;
            let mut kickers: Vec<u8> = rank_counts
                .iter()
                .filter(|(_, count)| *count == 1)
                .map(|(rank, _)| *rank)
                .collect();

            kickers.sort_by(|a, b| b.cmp(a));

            let mut result = vec![three_rank];
            result.extend(kickers);

            return PokerHandScore {
                category: 3,
                kickers: result,
            };
        }

        if rank_counts[0].1 == 2 && rank_counts[1].1 == 2 {
            let mut pair_ranks = vec![rank_counts[0].0, rank_counts[1].0];
            pair_ranks.sort_by(|a, b| b.cmp(a));

            let kicker = rank_counts
                .iter()
                .find(|(_, count)| *count == 1)
                .map(|(rank, _)| *rank)
                .expect("Two pair should have kicker");

            let mut result = pair_ranks;
            result.push(kicker);

            return PokerHandScore {
                category: 2,
                kickers: result,
            };
        }

        if rank_counts[0].1 == 2 {
            let pair_rank = rank_counts[0].0;
            let mut kickers: Vec<u8> = rank_counts
                .iter()
                .filter(|(_, count)| *count == 1)
                .map(|(rank, _)| *rank)
                .collect();

            kickers.sort_by(|a, b| b.cmp(a));

            let mut result = vec![pair_rank];
            result.extend(kickers);

            return PokerHandScore {
                category: 1,
                kickers: result,
            };
        }

        PokerHandScore {
            category: 0,
            kickers: ranks,
        }
    }

    fn straight_high_card(ranks_desc: &[u8]) -> Option<u8> {
        let mut unique = ranks_desc.to_vec();
        unique.sort();
        unique.dedup();
        unique.sort_by(|a, b| b.cmp(a));

        if unique.contains(&14) {
            unique.push(1);
        }

        for window in unique.windows(5) {
            if window[0] == window[1] + 1
                && window[1] == window[2] + 1
                && window[2] == window[3] + 1
                && window[3] == window[4] + 1
            {
                return Some(window[0]);
            }
        }

        None
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Only owner can call this method"
        );
    }

    fn assert_not_paused(&self) {
        assert!(!self.paused, "Contract is paused");
    }
}

fn table_buy_in_plus_storage(
    table_id: u64,
    contract: &Contract,
    storage_cost: Balance,
) -> Balance {
    let table = contract
        .tables
        .get(&table_id)
        .expect("Table does not exist");

    table.buy_in + storage_cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{testing_env_with_promise_results, VMContextBuilder};
    use near_sdk::testing_env;

    const ONE_NEAR: Balance = 1_000_000_000_000_000_000_000_000;

    fn account(name: &str) -> AccountId {
        name.parse::<AccountId>().unwrap()
    }

    fn set_context(predecessor: AccountId) {
        let context = VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .build();

        testing_env!(context);
    }

    fn set_context_with_deposit(predecessor: AccountId, deposit: Balance) {
        let context = VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .attached_deposit(near_sdk::NearToken::from_yoctonear(deposit))
            .build();

        testing_env!(context);
    }

    #[test]
    fn initializes_contract() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        assert_eq!(contract.get_owner(), owner);
        assert_eq!(contract.is_paused(), false);

        let range = contract.get_buy_in_range();
        assert_eq!(range.min_buy_in, ONE_NEAR);
        assert_eq!(range.max_buy_in, ONE_NEAR * 10);
    }

    #[test]
    fn owner_can_set_buy_in_range() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(owner);

        contract.set_buy_in_range(U128(ONE_NEAR * 2), U128(ONE_NEAR * 20));

        let range = contract.get_buy_in_range();
        assert_eq!(range.min_buy_in, ONE_NEAR * 2);
        assert_eq!(range.max_buy_in, ONE_NEAR * 20);
    }

    #[test]
    #[should_panic(expected = "Only owner can call this method")]
    fn non_owner_cannot_set_buy_in_range() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(alice);

        contract.set_buy_in_range(U128(ONE_NEAR * 2), U128(ONE_NEAR * 20));
    }

    #[test]
    #[should_panic(expected = "Minimum buy-in must be less than or equal to maximum buy-in")]
    fn invalid_buy_in_range_fails() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        Contract::new(owner, U128(ONE_NEAR * 10), U128(ONE_NEAR));
    }

    #[test]
    fn owner_can_pause() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(owner);

        contract.pause();

        assert_eq!(contract.is_paused(), true);
    }

    #[test]
    fn owner_can_unpause() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(owner.clone());

        contract.pause();
        assert_eq!(contract.is_paused(), true);

        set_context(owner);

        contract.unpause();

        assert_eq!(contract.is_paused(), false);
    }

    #[test]
    #[should_panic(expected = "Only owner can call this method")]
    fn non_owner_cannot_pause() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(alice);

        contract.pause();
    }

    #[test]
    #[should_panic(expected = "Only owner can call this method")]
    fn non_owner_cannot_unpause() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(owner);

        contract.pause();

        set_context(alice);

        contract.unpause();
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn assert_not_paused_fails_when_paused() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(owner);

        contract.pause();
        contract.assert_not_paused();
    }

    #[test]
    fn create_table_with_valid_buy_in_succeeds() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        assert_eq!(table_id, 0);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.id, 0);
        assert_eq!(table.creator_id, alice.clone());
        assert_eq!(table.buy_in, ONE_NEAR * 2);
        assert_eq!(table.players, vec![alice]);
        assert_eq!(table.status, TableStatus::WaitingForPlayers);
    }

    #[test]
    #[should_panic(expected = "Buy-in is below the minimum allowed")]
    fn create_table_with_invalid_low_buy_in_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR);

        contract.create_table(U128(ONE_NEAR / 2));
    }

    #[test]
    #[should_panic(expected = "Buy-in is above the maximum allowed")]
    fn create_table_with_invalid_high_buy_in_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR);

        contract.create_table(U128(ONE_NEAR * 20));
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit for buy-in and storage")]
    fn create_table_without_deposit_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(alice);

        contract.create_table(U128(ONE_NEAR * 2));
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn create_table_fails_when_paused() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context(owner);

        contract.pause();

        set_context_with_deposit(alice, ONE_NEAR);

        contract.create_table(U128(ONE_NEAR * 2));
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit for buy-in and storage")]
    fn create_table_with_insufficient_storage_deposit_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, 1);

        contract.create_table(U128(ONE_NEAR * 2));
    }

    #[test]
    fn create_table_with_excess_storage_deposit_succeeds() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.id, table_id);
        assert_eq!(table.creator_id, alice.clone());
        assert_eq!(table.players, vec![alice]);
        assert_eq!(table.status, TableStatus::WaitingForPlayers);
    }

    #[test]
    fn join_table_succeeds_for_second_player() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob.clone(), ONE_NEAR * 3);

        contract.join_table(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.players, vec![alice, bob]);
        assert_eq!(table.status, TableStatus::Active);
    }

    #[test]
    #[should_panic(expected = "Player already joined this table")]
    fn same_player_cannot_join_twice() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        contract.join_table(table_id);
    }

    #[test]
    #[should_panic(expected = "Table does not exist")]
    fn cannot_join_nonexistent_table() {
        let owner = account("owner.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(999);
    }

    #[test]
    #[should_panic(expected = "Table is not waiting for players")]
    fn cannot_join_active_table() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");
        let carol = account("carol.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);

        set_context_with_deposit(carol, ONE_NEAR * 3);

        contract.join_table(table_id);
    }

    #[test]
    #[should_panic(expected = "Attach buy-in plus storage deposit")]
    fn join_table_with_only_buy_in_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 2);

        contract.join_table(table_id);
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit for buy-in and storage")]
    fn join_table_with_insufficient_storage_deposit_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 2 + 1);

        contract.join_table(table_id);
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn join_table_fails_when_paused() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context(owner);

        contract.pause();

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);
    }

    #[test]
    fn game_starts_and_locks_order_when_second_player_joins() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        let table_before_join = contract.get_table(table_id).unwrap();

        assert_eq!(table_before_join.status, TableStatus::WaitingForPlayers);
        assert_eq!(table_before_join.order_locked, false);
        assert_eq!(table_before_join.current_turn_index, None);
        assert_eq!(table_before_join.started_at, None);

        set_context_with_deposit(bob.clone(), ONE_NEAR * 3);

        contract.join_table(table_id);

        let table_after_join = contract.get_table(table_id).unwrap();

        assert_eq!(table_after_join.players, vec![alice, bob]);
        assert_eq!(table_after_join.status, TableStatus::Active);
        assert_eq!(table_after_join.order_locked, true);
        assert_eq!(table_after_join.current_turn_index, Some(0));
        assert!(table_after_join.started_at.is_some());
    }

    #[test]
    fn current_turn_is_first_player_after_start() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);

        let table = contract.get_table(table_id).unwrap();

        let current_turn_index = table.current_turn_index.unwrap() as usize;
        let current_player = table.players[current_turn_index].clone();

        assert_eq!(current_player, alice);
    }

    #[test]
    #[should_panic(expected = "Table is not waiting for players")]
    fn new_players_cannot_join_after_game_starts() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");
        let carol = account("carol.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);

        set_context_with_deposit(carol, ONE_NEAR * 3);

        contract.join_table(table_id);
    }

    #[test]
    fn start_game_deals_two_cards_to_each_player() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob.clone(), ONE_NEAR * 3);

        contract.join_table(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.player_cards.len(), 2);

        for hand in table.player_cards.iter() {
            assert_eq!(hand.cards.len(), 2);
        }

        assert_eq!(table.player_cards[0].player_id, alice);
        assert_eq!(table.player_cards[1].player_id, bob);
    }

    #[test]
    fn dealt_cards_are_unique() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);

        let table = contract.get_table(table_id).unwrap();

        let mut seen_cards = HashSet::new();

        for hand in table.player_cards.iter() {
            for card in hand.cards.iter() {
                assert!(
                    seen_cards.insert(card.clone()),
                    "Duplicate card was dealt"
                );
            }
        }
    }

    #[test]
    fn deck_has_correct_remaining_card_count() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.remaining_deck_count, 48);
    }

    #[test]
    fn community_cards_start_empty() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice, ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob, ONE_NEAR * 3);

        contract.join_table(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.community_cards.len(), 0);
    }

    fn setup_active_table() -> (Contract, u64, AccountId, AccountId) {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob.clone(), ONE_NEAR * 3);

        contract.join_table(table_id);

        (contract, table_id, alice, bob)
    }

    #[test]
    fn current_player_can_submit_action() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(table_id, PlayerAction::Check);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.action_history.len(), 1);
        assert_eq!(table.action_history[0].player_id, alice);
        assert_eq!(table.action_history[0].action, PlayerAction::Check);
    }

    #[test]
    #[should_panic(expected = "Only the current player can act")]
    fn wrong_player_cannot_act() {
        let (mut contract, table_id, _, bob) = setup_active_table();

        set_context(bob);

        contract.submit_action(table_id, PlayerAction::Check);
    }

    #[test]
    #[should_panic(expected = "Only table players can submit actions")]
    fn non_player_cannot_act() {
        let (mut contract, table_id, _, _) = setup_active_table();
        let carol = account("carol.testnet");

        set_context(carol);

        contract.submit_action(table_id, PlayerAction::Check);
    }

    #[test]
    fn action_moves_turn_to_next_player() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Check);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.current_turn_index, Some(1));

        let current_player = table.players[table.current_turn_index.unwrap() as usize].clone();

        assert_eq!(current_player, bob);
    }

    #[test]
    #[should_panic(expected = "Table is not active")]
    fn cannot_submit_action_on_waiting_table() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Check);
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn cannot_submit_action_when_paused() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        let owner = contract.get_owner();

        set_context(owner);

        contract.pause();

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Check);
    }

    fn get_player_balance(table: &TableView, player_id: &AccountId) -> Balance {
        table
            .player_balances
            .iter()
            .find(|balance| &balance.player_id == player_id)
            .expect("Player balance should exist")
            .balance
    }

    #[test]
    fn game_starts_with_player_internal_balances_after_blinds() {
        let (contract, table_id, alice, bob) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.player_balances.len(), 2);
        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - SMALL_BLIND
        );
        assert_eq!(
            get_player_balance(&table, &bob),
            ONE_NEAR * 2 - BIG_BLIND
        );
    }

    #[test]
    fn game_starts_with_blinds_in_pot() {
        let (contract, table_id, _, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, SMALL_BLIND + BIG_BLIND);
    }

    #[test]
    fn raise_decreases_current_player_balance() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - SMALL_BLIND - ONE_NEAR / 2
        );
    }

    #[test]
    fn raise_increases_pot() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, SMALL_BLIND + BIG_BLIND + ONE_NEAR / 2);
    }

    #[test]
    #[should_panic(expected = "Raise amount exceeds player balance")]
    fn raise_larger_than_balance_fails() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR * 3),
            },
        );
    }

    #[test]
    #[should_panic(expected = "Raise amount must be greater than zero")]
    fn raise_zero_fails() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(0),
            },
        );
    }

    #[test]
    fn check_does_not_change_balance_or_pot() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(table_id, PlayerAction::Check);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, SMALL_BLIND + BIG_BLIND);
        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - SMALL_BLIND
        );
        assert_eq!(
            get_player_balance(&table, &bob),
            ONE_NEAR * 2 - BIG_BLIND
        );
    }

    #[test]
    fn owner_can_resolve_round() {
        let (mut contract, table_id, alice, _) = setup_active_table();
        let owner = contract.get_owner();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        set_context(owner);

        contract.resolve_round(table_id, alice.clone());

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.status, TableStatus::Finished);
        assert!(table.round_result.is_some());
        assert_eq!(table.round_result.unwrap().winner_id, alice);
    }

    #[test]
    #[should_panic(expected = "Only owner can call this method")]
    fn non_owner_cannot_resolve_round() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        set_context(bob);

        contract.resolve_round(table_id, alice);
    }

    #[test]
    fn winner_balance_increases_by_pot() {
        let (mut contract, table_id, alice, _) = setup_active_table();
        let owner = contract.get_owner();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        let before_resolution = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&before_resolution, &alice),
            ONE_NEAR * 2 - SMALL_BLIND - ONE_NEAR / 2
        );

        set_context(owner);

        contract.resolve_round(table_id, alice.clone());

        let after_resolution = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&after_resolution, &alice),
            ONE_NEAR * 2 + BIG_BLIND
        );
    }

    #[test]
    fn pot_resets_after_resolution() {
        let (mut contract, table_id, alice, _) = setup_active_table();
        let owner = contract.get_owner();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        set_context(owner);

        contract.resolve_round(table_id, alice);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, 0);
    }

    #[test]
    #[should_panic(expected = "Winner must be a table player")]
    fn cannot_resolve_with_non_player_winner() {
        let (mut contract, table_id, alice, _) = setup_active_table();
        let owner = contract.get_owner();
        let carol = account("carol.testnet");

        set_context(alice);

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        set_context(owner);

        contract.resolve_round(table_id, carol);
    }

    #[test]
    #[should_panic(expected = "Table is not active")]
    fn cannot_resolve_waiting_table() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(
            owner.clone(),
            U128(ONE_NEAR),
            U128(ONE_NEAR * 10),
        );

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context(owner);

        contract.resolve_round(table_id, alice);
    }

    #[test]
    #[should_panic(expected = "Table is not active")]
    fn cannot_resolve_round_twice() {
        let (mut contract, table_id, alice, _) = setup_active_table();
        let owner = contract.get_owner();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        set_context(owner.clone());

        contract.resolve_round(table_id, alice.clone());

        set_context(owner);

        contract.resolve_round(table_id, alice);
    }

    fn contract_account() -> AccountId {
        account("contract.testnet")
    }

    fn set_callback_context_with_result(result: PromiseResult) {
        let current = contract_account();

        let context = VMContextBuilder::new()
            .current_account_id(current.clone())
            .predecessor_account_id(current)
            .build();

        testing_env_with_promise_results(context, result);
    }

    fn setup_finished_table_with_pot() -> (Contract, u64, AccountId, AccountId) {
        let (mut contract, table_id, alice, bob) = setup_active_table();
        let owner = contract.get_owner();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR / 2),
            },
        );

        set_context(owner);

        contract.resolve_round(table_id, alice.clone());

        (contract, table_id, alice, bob)
    }

    #[test]
    fn player_can_withdraw_after_finished_round() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice.clone()).unwrap();

        assert_eq!(pending.table_id, table_id);
        assert_eq!(pending.player_id, alice);
        assert!(pending.amount > 0);
    }

    #[test]
    fn withdraw_deducts_internal_balance_before_transfer() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());

        contract.withdraw(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(get_player_balance(&table, &alice), 0);
    }

    #[test]
    #[should_panic(expected = "Withdrawals are only allowed after the table is finished")]
    fn cannot_withdraw_from_active_table() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.withdraw(table_id);
    }

    #[test]
    #[should_panic(expected = "No balance available to withdraw")]
    fn cannot_withdraw_zero_balance() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice.clone()).unwrap();

        set_callback_context_with_result(PromiseResult::Successful(Vec::new()));

        contract.on_withdraw_complete(
            pending.player_id.clone(),
            pending.table_id,
            pending.amount,
        );

        set_context(alice);

        contract.withdraw(table_id);
    }

    #[test]
    #[should_panic(expected = "Only table players can withdraw")]
    fn non_player_cannot_withdraw() {
        let (mut contract, table_id, _, _) = setup_finished_table_with_pot();
        let carol = account("carol.testnet");

        set_context(carol);

        contract.withdraw(table_id);
    }

    #[test]
    fn withdraw_callback_success_clears_pending_withdrawal() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice.clone()).unwrap();

        set_callback_context_with_result(PromiseResult::Successful(Vec::new()));

        let success = contract.on_withdraw_complete(
            pending.player_id.clone(),
            pending.table_id,
            pending.amount,
        );

        assert_eq!(success, true);
        assert!(contract.get_pending_withdrawal(alice).is_none());
    }

    #[test]
    fn withdraw_callback_failure_restores_balance() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice.clone()).unwrap();

        let table_after_withdraw = contract.get_table(table_id).unwrap();

        assert_eq!(get_player_balance(&table_after_withdraw, &alice), 0);

        set_callback_context_with_result(PromiseResult::Failed);

        let success = contract.on_withdraw_complete(
            pending.player_id.clone(),
            pending.table_id,
            pending.amount,
        );

        assert_eq!(success, false);

        let table_after_callback = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&table_after_callback, &alice),
            pending.amount
        );

        assert!(contract.get_pending_withdrawal(alice).is_none());
    }

    fn set_context_with_timestamp(predecessor: AccountId, timestamp: u64) {
        let context = VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .block_timestamp(timestamp)
            .build();

        testing_env!(context);
    }

    #[test]
    #[should_panic(expected = "Timeout has not passed yet")]
    fn cannot_claim_timeout_refund_before_timeout() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();
        let last_action_at = table.last_action_at.unwrap();

        set_context_with_timestamp(alice, last_action_at + ABANDON_TIMEOUT_NS - 1);

        contract.claim_timeout_refund(table_id);
    }

    #[test]
    fn can_claim_timeout_refund_after_timeout() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();
        let last_action_at = table.last_action_at.unwrap();

        set_context_with_timestamp(alice, last_action_at + ABANDON_TIMEOUT_NS);

        contract.claim_timeout_refund(table_id);

        let table_after = contract.get_table(table_id).unwrap();

        assert_eq!(table_after.status, TableStatus::Cancelled);
        assert_eq!(table_after.current_turn_index, None);
    }

    #[test]
    #[should_panic(expected = "Only table players can claim timeout refund")]
    fn non_player_cannot_claim_timeout_refund() {
        let (mut contract, table_id, _, _) = setup_active_table();
        let carol = account("carol.testnet");

        let table = contract.get_table(table_id).unwrap();
        let last_action_at = table.last_action_at.unwrap();

        set_context_with_timestamp(carol, last_action_at + ABANDON_TIMEOUT_NS);

        contract.claim_timeout_refund(table_id);
    }

    #[test]
    fn timeout_refund_moves_pot_back_to_player_balances() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: U128(ONE_NEAR),
            },
        );

        let table_before = contract.get_table(table_id).unwrap();

        assert_eq!(table_before.pot, SMALL_BLIND + BIG_BLIND + ONE_NEAR);
        let last_action_at = table_before.last_action_at.unwrap();

        set_context_with_timestamp(bob.clone(), last_action_at + ABANDON_TIMEOUT_NS);

        contract.claim_timeout_refund(table_id);

        let table_after = contract.get_table(table_id).unwrap();

        assert_eq!(table_after.pot, 0);
        assert_eq!(table_after.status, TableStatus::Cancelled);

        assert_eq!(
            get_player_balance(&table_after, &alice),
            ONE_NEAR * 2 - SMALL_BLIND - ONE_NEAR
                + (SMALL_BLIND + BIG_BLIND + ONE_NEAR) / 2
        );

        assert_eq!(
            get_player_balance(&table_after, &bob),
            ONE_NEAR * 2 - BIG_BLIND
                + (SMALL_BLIND + BIG_BLIND + ONE_NEAR) / 2
        );
    }

    #[test]
    fn can_withdraw_after_timeout_cancelled_table() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();
        let last_action_at = table.last_action_at.unwrap();

        set_context_with_timestamp(alice.clone(), last_action_at + ABANDON_TIMEOUT_NS);

        contract.claim_timeout_refund(table_id);

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice).unwrap();

        assert_eq!(pending.table_id, table_id);
        assert!(pending.amount > 0);
    }

    #[test]
    fn get_open_tables_returns_waiting_tables_only() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");
        let bob = account("bob.testnet");
        let carol = account("carol.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, U128(ONE_NEAR), U128(ONE_NEAR * 10));

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);
        let waiting_table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(bob.clone(), ONE_NEAR * 3);
        let active_table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context_with_deposit(carol, ONE_NEAR * 3);
        contract.join_table(active_table_id);

        let open_tables = contract.get_open_tables();

        assert_eq!(open_tables.len(), 1);
        assert_eq!(open_tables[0].id, waiting_table_id);
        assert_eq!(open_tables[0].status, TableStatus::WaitingForPlayers);
    }

    #[test]
    fn get_player_balance_returns_expected_balance() {
        let (contract, table_id, alice, _) = setup_active_table();

        let balance = contract
            .get_player_balance(table_id, alice)
            .expect("Balance should exist");

        assert_eq!(balance, ONE_NEAR * 2 - SMALL_BLIND);
    }

    #[test]
    fn get_player_balance_returns_none_for_non_player() {
        let (contract, table_id, _, _) = setup_active_table();
        let carol = account("carol.testnet");

        let balance = contract.get_player_balance(table_id, carol);

        assert_eq!(balance, None);
    }

    #[test]
    fn get_current_turn_returns_first_player_initially() {
        let (contract, table_id, alice, _) = setup_active_table();

        let turn = contract
            .get_current_turn(table_id)
            .expect("Turn should exist");

        assert_eq!(turn.current_turn_index, Some(0));
        assert_eq!(turn.current_player, Some(alice));
    }

    #[test]
    fn get_game_state_returns_expected_state() {
        let (contract, table_id, alice, bob) = setup_active_table();

        let state = contract
            .get_game_state(table_id)
            .expect("Game state should exist");

        assert_eq!(state.table_id, table_id);
        assert_eq!(state.status, TableStatus::Active);
        assert_eq!(state.players, vec![alice, bob]);
        assert_eq!(state.current_turn_index, Some(0));
        assert_eq!(state.pot, SMALL_BLIND + BIG_BLIND);
        assert_eq!(state.community_cards.len(), 0);
        assert_eq!(state.remaining_deck_count, 48);
        assert_eq!(state.game_stage, GameStage::PreFlop);
    }

    #[test]
    #[should_panic(expected = "Player already has a pending withdrawal")]
    fn pending_withdrawal_blocks_second_withdraw() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());
        contract.withdraw(table_id);
        set_context(alice);
        contract.withdraw(table_id);
    }

    #[test]
    #[should_panic(expected = "Pending withdrawal amount mismatch")]
    fn withdraw_callback_amount_mismatch_fails() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());
        contract.withdraw(table_id);
        let pending = contract.get_pending_withdrawal(alice).unwrap();
        set_callback_context_with_result(PromiseResult::Successful(Vec::new()));
        contract.on_withdraw_complete(
            pending.player_id,
            pending.table_id,
            pending.amount + 1,
        );
    }

    #[test]
    #[should_panic(expected = "Pending withdrawal table mismatch")]
    fn withdraw_callback_table_mismatch_fails() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());
        contract.withdraw(table_id);
        let pending = contract.get_pending_withdrawal(alice).unwrap();
        set_callback_context_with_result(PromiseResult::Successful(Vec::new()));
        contract.on_withdraw_complete(
            pending.player_id,
            pending.table_id + 1,
            pending.amount,
        );
    }

    #[test]
    fn cancelled_table_allows_withdraw() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();
        let last_action_at = table.last_action_at.unwrap();

        set_context_with_timestamp(
            alice.clone(),
            last_action_at + ABANDON_TIMEOUT_NS,
        );

        contract.claim_timeout_refund(table_id);

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice).unwrap();

        assert_eq!(pending.table_id, table_id);
        assert!(pending.amount > 0);
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn paused_contract_blocks_timeout_refund() {
        let (mut contract, table_id, alice, _) = setup_active_table();
        let owner = contract.get_owner();

        let table = contract.get_table(table_id).unwrap();
        let last_action_at = table.last_action_at.unwrap();

        set_context(owner);

        contract.pause();

        set_context_with_timestamp(
            alice,
            last_action_at + ABANDON_TIMEOUT_NS,
        );

        contract.claim_timeout_refund(table_id);
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn paused_contract_blocks_withdraw() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();
        let owner = contract.get_owner();

        set_context(owner);

        contract.pause();

        set_context(alice);

        contract.withdraw(table_id);
    }

    #[test]
    fn failed_withdraw_callback_allows_retry() {
        let (mut contract, table_id, alice, _) = setup_finished_table_with_pot();

        set_context(alice.clone());

        contract.withdraw(table_id);

        let pending = contract.get_pending_withdrawal(alice.clone()).unwrap();

        set_callback_context_with_result(PromiseResult::Failed);

        let success = contract.on_withdraw_complete(
            pending.player_id.clone(),
            pending.table_id,
            pending.amount,
        );

        assert_eq!(success, false);
        assert!(contract.get_pending_withdrawal(alice.clone()).is_none());

        set_context(alice.clone());

        contract.withdraw(table_id);

        let retry_pending = contract.get_pending_withdrawal(alice).unwrap();

        assert_eq!(retry_pending.table_id, table_id);
        assert_eq!(retry_pending.amount, pending.amount);
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit for buy-in and storage")]
    fn create_table_with_storage_only_deposit_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(
            owner,
            U128(ONE_NEAR),
            U128(ONE_NEAR * 10),
        );

        set_context_with_deposit(alice, ONE_NEAR / 10);

        contract.create_table(U128(ONE_NEAR * 2));
    }

    #[test]
    fn create_table_with_buy_in_plus_storage_deposit_succeeds() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(
            owner,
            U128(ONE_NEAR),
            U128(ONE_NEAR * 10),
        );

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.id, table_id);
        assert_eq!(table.creator_id, alice.clone());
        assert_eq!(table.players, vec![alice]);
        assert_eq!(table.buy_in, ONE_NEAR * 2);
        assert_eq!(table.status, TableStatus::WaitingForPlayers);
    }

    #[test]
    fn game_sets_small_and_big_blind_indices() {
        let (contract, table_id, _, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.small_blind_index, Some(0));
        assert_eq!(table.big_blind_index, Some(1));
    }

    #[test]
    fn small_and_big_blinds_are_recorded_on_table() {
        let (contract, table_id, _, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.small_blind, SMALL_BLIND);
        assert_eq!(table.big_blind, BIG_BLIND);
    }

    #[test]
    fn blinds_are_deducted_from_correct_players() {
        let (contract, table_id, alice, bob) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - SMALL_BLIND
        );

        assert_eq!(
            get_player_balance(&table, &bob),
            ONE_NEAR * 2 - BIG_BLIND
        );
    }

    #[test]
    fn game_starts_at_preflop() {
        let (contract, table_id, _, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.game_stage, GameStage::PreFlop);
        assert_eq!(table.community_cards.len(), 0);
    }

    #[test]
    fn advance_stage_deals_flop() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.advance_stage(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.game_stage, GameStage::Flop);
        assert_eq!(table.community_cards.len(), 3);
        assert_eq!(table.remaining_deck_count, 45);
    }

    #[test]
    fn advance_stage_deals_turn() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice);
        contract.advance_stage(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.game_stage, GameStage::Turn);
        assert_eq!(table.community_cards.len(), 4);
        assert_eq!(table.remaining_deck_count, 44);
    }

    #[test]
    fn advance_stage_deals_river() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice);
        contract.advance_stage(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.game_stage, GameStage::River);
        assert_eq!(table.community_cards.len(), 5);
        assert_eq!(table.remaining_deck_count, 43);
    }

    #[test]
    fn advance_stage_moves_river_to_showdown_without_drawing_card() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice);
        contract.advance_stage(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.game_stage, GameStage::Showdown);
        assert_eq!(table.community_cards.len(), 5);
        assert_eq!(table.remaining_deck_count, 43);
    }

    #[test]
    #[should_panic(expected = "Game is already at showdown")]
    fn cannot_advance_stage_after_showdown() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice.clone());
        contract.advance_stage(table_id);

        set_context(alice);
        contract.advance_stage(table_id);
    }

    #[test]
    #[should_panic(expected = "Only table players or owner can advance stage")]
    fn non_player_cannot_advance_stage() {
        let (mut contract, table_id, _, _) = setup_active_table();
        let carol = account("carol.testnet");

        set_context(carol);

        contract.advance_stage(table_id);
    }

    #[test]
    fn fold_finishes_table() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Fold);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.status, TableStatus::Finished);
        assert_eq!(table.current_turn_index, None);
    }

    #[test]
    fn fold_awards_pot_to_opponent() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Fold);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&table, &bob),
            ONE_NEAR * 2 - BIG_BLIND + SMALL_BLIND + BIG_BLIND
        );
    }

    #[test]
    fn fold_resets_pot() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Fold);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, 0);
    }

    #[test]
    fn fold_records_round_result() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Fold);

        let table = contract.get_table(table_id).unwrap();
        let result = table.round_result.expect("Round result should exist");

        assert_eq!(result.winner_id, bob);
        assert_eq!(result.pot_awarded, SMALL_BLIND + BIG_BLIND);
    }

    #[test]
    #[should_panic(expected = "Only the current player can act")]
    fn wrong_player_cannot_fold() {
        let (mut contract, table_id, _, bob) = setup_active_table();

        set_context(bob);

        contract.submit_action(table_id, PlayerAction::Fold);
    }

    #[test]
    #[should_panic(expected = "Table is not active")]
    fn cannot_fold_on_waiting_table() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(
            owner,
            U128(ONE_NEAR),
            U128(ONE_NEAR * 10),
        );

        set_context_with_deposit(alice.clone(), ONE_NEAR * 3);

        let table_id = contract.create_table(U128(ONE_NEAR * 2));

        set_context(alice);

        contract.submit_action(table_id, PlayerAction::Fold);
    }

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    fn evaluator_straight_beats_pair() {
        let straight_cards = vec![
            card(Rank::Ten, Suit::Clubs),
            card(Rank::Jack, Suit::Diamonds),
            card(Rank::Queen, Suit::Hearts),
            card(Rank::King, Suit::Spades),
            card(Rank::Ace, Suit::Clubs),
            card(Rank::Two, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
        ];

        let pair_cards = vec![
            card(Rank::Ace, Suit::Clubs),
            card(Rank::Ace, Suit::Diamonds),
            card(Rank::King, Suit::Hearts),
            card(Rank::Queen, Suit::Spades),
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Two, Suit::Hearts),
        ];

        let straight_score = Contract::best_hand_score(&straight_cards);
        let pair_score = Contract::best_hand_score(&pair_cards);

        assert!(straight_score > pair_score);
        assert_eq!(straight_score.category, 4);
        assert_eq!(pair_score.category, 1);
    }

    #[test]
    fn evaluator_flush_beats_straight() {
        let flush_cards = vec![
            card(Rank::Ace, Suit::Hearts),
            card(Rank::Ten, Suit::Hearts),
            card(Rank::Eight, Suit::Hearts),
            card(Rank::Five, Suit::Hearts),
            card(Rank::Two, Suit::Hearts),
            card(Rank::King, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
        ];

        let straight_cards = vec![
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Ten, Suit::Diamonds),
            card(Rank::Jack, Suit::Hearts),
            card(Rank::Queen, Suit::Spades),
            card(Rank::King, Suit::Clubs),
            card(Rank::Two, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
        ];

        let flush_score = Contract::best_hand_score(&flush_cards);
        let straight_score = Contract::best_hand_score(&straight_cards);

        assert!(flush_score > straight_score);
        assert_eq!(flush_score.category, 5);
        assert_eq!(straight_score.category, 4);
    }

    #[test]
    fn evaluator_full_house_beats_flush() {
        let full_house_cards = vec![
            card(Rank::King, Suit::Clubs),
            card(Rank::King, Suit::Diamonds),
            card(Rank::King, Suit::Hearts),
            card(Rank::Two, Suit::Spades),
            card(Rank::Two, Suit::Clubs),
            card(Rank::Ace, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
        ];

        let flush_cards = vec![
            card(Rank::Ace, Suit::Hearts),
            card(Rank::Ten, Suit::Hearts),
            card(Rank::Eight, Suit::Hearts),
            card(Rank::Five, Suit::Hearts),
            card(Rank::Two, Suit::Hearts),
            card(Rank::King, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
        ];

        let full_house_score = Contract::best_hand_score(&full_house_cards);
        let flush_score = Contract::best_hand_score(&flush_cards);

        assert!(full_house_score > flush_score);
        assert_eq!(full_house_score.category, 6);
        assert_eq!(flush_score.category, 5);
    }

    #[test]
    fn evaluator_straight_flush_beats_four_of_a_kind() {
        let straight_flush_cards = vec![
            card(Rank::Nine, Suit::Spades),
            card(Rank::Ten, Suit::Spades),
            card(Rank::Jack, Suit::Spades),
            card(Rank::Queen, Suit::Spades),
            card(Rank::King, Suit::Spades),
            card(Rank::Two, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
        ];

        let four_kind_cards = vec![
            card(Rank::Ace, Suit::Clubs),
            card(Rank::Ace, Suit::Diamonds),
            card(Rank::Ace, Suit::Hearts),
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Clubs),
            card(Rank::Two, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
        ];

        let straight_flush_score = Contract::best_hand_score(&straight_flush_cards);
        let four_kind_score = Contract::best_hand_score(&four_kind_cards);

        assert!(straight_flush_score > four_kind_score);
        assert_eq!(straight_flush_score.category, 8);
        assert_eq!(four_kind_score.category, 7);
    }

    #[test]
    fn evaluator_supports_ace_low_straight() {
        let cards = vec![
            card(Rank::Ace, Suit::Clubs),
            card(Rank::Two, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
            card(Rank::Four, Suit::Spades),
            card(Rank::Five, Suit::Clubs),
            card(Rank::King, Suit::Diamonds),
            card(Rank::Nine, Suit::Hearts),
        ];

        let score = Contract::best_hand_score(&cards);

        assert_eq!(score.category, 4);
        assert_eq!(score.kickers, vec![5]);
    }

    fn force_showdown_with_cards(
        contract: &mut Contract,
        table_id: u64,
        player_cards: Vec<PlayerCards>,
        community_cards: Vec<Card>,
        pot: Balance,
    ) {
        let mut table = contract
            .tables
            .get(&table_id)
            .expect("Table should exist")
            .clone();

        table.game_stage = GameStage::Showdown;
        table.community_cards = community_cards;
        table.player_cards = player_cards;
        table.pot = pot;

        contract.tables.insert(table_id, table);
    }

    #[test]
    #[should_panic(expected = "Round can only be evaluated at showdown")]
    fn resolve_by_evaluation_requires_showdown() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.resolve_round_by_evaluation(table_id);
    }

    #[test]
    #[should_panic(expected = "Evaluation requires five community cards")]
    fn resolve_by_evaluation_requires_five_community_cards() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        force_showdown_with_cards(
            &mut contract,
            table_id,
            vec![
                PlayerCards {
                    player_id: alice.clone(),
                    cards: vec![
                        card(Rank::Ace, Suit::Spades),
                        card(Rank::Ace, Suit::Hearts),
                    ],
                },
                PlayerCards {
                    player_id: bob.clone(),
                    cards: vec![
                        card(Rank::King, Suit::Spades),
                        card(Rank::King, Suit::Hearts),
                    ],
                },
            ],
            vec![
                card(Rank::Two, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
            ],
            SMALL_BLIND + BIG_BLIND,
        );

        set_context(alice);

        contract.resolve_round_by_evaluation(table_id);
    }

    #[test]
    fn resolve_by_evaluation_awards_pot_to_best_hand() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        let pot = SMALL_BLIND + BIG_BLIND;

        force_showdown_with_cards(
            &mut contract,
            table_id,
            vec![
                PlayerCards {
                    player_id: alice.clone(),
                    cards: vec![
                        card(Rank::Ace, Suit::Spades),
                        card(Rank::Ace, Suit::Hearts),
                    ],
                },
                PlayerCards {
                    player_id: bob.clone(),
                    cards: vec![
                        card(Rank::King, Suit::Spades),
                        card(Rank::King, Suit::Hearts),
                    ],
                },
            ],
            vec![
                card(Rank::Two, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Nine, Suit::Clubs),
                card(Rank::Ten, Suit::Diamonds),
            ],
            pot,
        );

        set_context(alice.clone());

        contract.resolve_round_by_evaluation(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.status, TableStatus::Finished);
        assert_eq!(table.pot, 0);
        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - SMALL_BLIND + pot
        );
        assert_eq!(table.round_result.unwrap().winner_id, alice);
    }

    #[test]
    fn resolve_by_evaluation_records_round_result() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        let pot = SMALL_BLIND + BIG_BLIND;

        force_showdown_with_cards(
            &mut contract,
            table_id,
            vec![
                PlayerCards {
                    player_id: alice.clone(),
                    cards: vec![
                        card(Rank::Queen, Suit::Spades),
                        card(Rank::Queen, Suit::Hearts),
                    ],
                },
                PlayerCards {
                    player_id: bob.clone(),
                    cards: vec![
                        card(Rank::King, Suit::Spades),
                        card(Rank::King, Suit::Hearts),
                    ],
                },
            ],
            vec![
                card(Rank::Two, Suit::Clubs),
                card(Rank::Three, Suit::Diamonds),
                card(Rank::Four, Suit::Hearts),
                card(Rank::Nine, Suit::Clubs),
                card(Rank::Ten, Suit::Diamonds),
            ],
            pot,
        );

        set_context(alice);

        contract.resolve_round_by_evaluation(table_id);

        let table = contract.get_table(table_id).unwrap();
        let result = table.round_result.expect("Round result should exist");

        assert_eq!(result.winner_id, bob);
        assert_eq!(result.pot_awarded, pot);
    }

    #[test]
    fn resolve_by_evaluation_splits_pot_on_tie() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        let pot = SMALL_BLIND + BIG_BLIND;

        force_showdown_with_cards(
            &mut contract,
            table_id,
            vec![
                PlayerCards {
                    player_id: alice.clone(),
                    cards: vec![
                        card(Rank::Two, Suit::Spades),
                        card(Rank::Seven, Suit::Hearts),
                    ],
                },
                PlayerCards {
                    player_id: bob.clone(),
                    cards: vec![
                        card(Rank::Three, Suit::Spades),
                        card(Rank::Eight, Suit::Hearts),
                    ],
                },
            ],
            vec![
                card(Rank::Ace, Suit::Clubs),
                card(Rank::King, Suit::Diamonds),
                card(Rank::Queen, Suit::Hearts),
                card(Rank::Jack, Suit::Spades),
                card(Rank::Ten, Suit::Clubs),
            ],
            pot,
        );

        set_context(alice.clone());

        contract.resolve_round_by_evaluation(table_id);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.status, TableStatus::Finished);
        assert_eq!(table.pot, 0);

        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - SMALL_BLIND + pot / 2
        );

        assert_eq!(
            get_player_balance(&table, &bob),
            ONE_NEAR * 2 - BIG_BLIND + pot / 2
        );

        assert_eq!(
            table.round_result.unwrap().winner_id,
            "split-pot.testnet".parse::<AccountId>().unwrap()
        );
    }
}
