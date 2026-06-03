use near_sdk::{
    borsh, env, near, AccountId, BorshStorageKey, NearToken, PanicOnDefault, Promise,
};
use near_sdk::store::UnorderedMap;
use std::collections::HashSet;

const TABLE_STORAGE_OVERHEAD_BYTES: u64 = 256;
const MAX_PLAYERS: usize = 2;

pub type Balance = u128;

#[derive(BorshStorageKey)]
#[near(serializers = [borsh])]
pub enum StorageKey {
    Tables,
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
    Raise { amount: Balance },
    Fold,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct ActionRecord {
    pub player_id: AccountId,
    pub action: PlayerAction,
    pub timestamp: u64,
}

#[derive(Clone)]
#[near(serializers = [borsh])]
pub struct Table {
    pub id: u64,
    pub creator_id: AccountId,
    pub buy_in: Balance,
    pub players: Vec<AccountId>,
    pub status: TableStatus,
    pub created_at: u64,
    pub order_locked: bool,
    pub current_turn_index: Option<u8>,
    pub started_at: Option<u64>,
    pub deck: Vec<Card>,
    pub player_cards: Vec<PlayerCards>,
    pub community_cards: Vec<Card>,
    pub action_history: Vec<ActionRecord>,
    pub pot: Balance,
    pub player_balances: Vec<PlayerBalance>,
}

#[near(serializers = [json])]
pub struct TableView {
    pub id: u64,
    pub creator_id: AccountId,
    pub buy_in: Balance,
    pub players: Vec<AccountId>,
    pub status: TableStatus,
    pub created_at: u64,
    pub order_locked: bool,
    pub current_turn_index: Option<u8>,
    pub started_at: Option<u64>,
    pub player_cards: Vec<PlayerCards>,
    pub community_cards: Vec<Card>,
    pub remaining_deck_count: usize,
    pub action_history: Vec<ActionRecord>,
    pub pot: Balance,
    pub player_balances: Vec<PlayerBalance>,
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
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, min_buy_in: Balance, max_buy_in: Balance) -> Self {
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

    pub fn set_buy_in_range(&mut self, min_buy_in: Balance, max_buy_in: Balance) {
        self.assert_owner();

        assert!(min_buy_in > 0, "Minimum buy-in must be greater than zero");
        assert!(
            min_buy_in <= max_buy_in,
            "Minimum buy-in must be less than or equal to maximum buy-in"
        );

        self.min_buy_in = min_buy_in;
        self.max_buy_in = max_buy_in;
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
    pub fn create_table(&mut self, buy_in: Balance) -> u64 {
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
            created_at: env::block_timestamp(),
            order_locked: false,
            current_turn_index: None,
            started_at: None,
            deck: Vec::new(),
            player_cards: Vec::new(),
            community_cards: Vec::new(),
            action_history: Vec::new(),
            pot: 0,
            player_balances: Vec::new(),
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

        assert!(
            attached_deposit >= storage_cost,
            "Insufficient deposit to cover storage"
        );

        let refund = attached_deposit - storage_cost;

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

        match &action {
            PlayerAction::Raise { amount } => {
                assert!(*amount > 0, "Raise amount must be greater than zero");

                let player_balance = table
                    .player_balances
                    .iter_mut()
                    .find(|balance| balance.player_id == actor_id)
                    .expect("Player balance does not exist");

                assert!(
                    player_balance.balance >= *amount,
                    "Raise amount exceeds player balance"
                );

                player_balance.balance -= *amount;
                table.pot += *amount;
            }
            PlayerAction::Check | PlayerAction::Call | PlayerAction::Fold => {}
        }

        table.action_history.push(ActionRecord {
            player_id: actor_id,
            action,
            timestamp: env::block_timestamp(),
        });

        table.current_turn_index =
            Some(((current_turn_index + 1) % table.players.len()) as u8);

        self.tables.insert(table_id, table);
    }

    pub fn get_table(&self, table_id: u64) -> Option<TableView> {
        self.tables.get(&table_id).map(|table| TableView {
            id: table.id,
            creator_id: table.creator_id.clone(),
            buy_in: table.buy_in,
            players: table.players.clone(),
            status: table.status.clone(),
            created_at: table.created_at,
            order_locked: table.order_locked,
            current_turn_index: table.current_turn_index,
            started_at: table.started_at,
            player_cards: table.player_cards.clone(),
            community_cards: table.community_cards.clone(),
            remaining_deck_count: table.deck.len(),
            action_history: table.action_history.clone(),
            pot: table.pot,
            player_balances: table.player_balances.clone(),
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

        table.status = TableStatus::Active;
        table.order_locked = true;
        table.current_turn_index = Some(0);
        table.started_at = Some(env::block_timestamp());
        table.deck = deck;
        table.player_cards = player_cards;
        table.community_cards = Vec::new();
        table.pot = 0;
        table.player_balances = player_balances;
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
    use near_sdk::test_utils::VMContextBuilder;
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

        let contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

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

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

        set_context(owner);

        contract.set_buy_in_range(ONE_NEAR * 2, ONE_NEAR * 20);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context(alice);

        contract.set_buy_in_range(ONE_NEAR * 2, ONE_NEAR * 20);
    }

    #[test]
    #[should_panic(expected = "Minimum buy-in must be less than or equal to maximum buy-in")]
    fn invalid_buy_in_range_fails() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        Contract::new(owner, ONE_NEAR * 10, ONE_NEAR);
    }

    #[test]
    fn owner_can_pause() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

        set_context(owner);

        contract.pause();

        assert_eq!(contract.is_paused(), true);
    }

    #[test]
    fn owner_can_unpause() {
        let owner = account("owner.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context(alice);

        contract.pause();
    }

    #[test]
    #[should_panic(expected = "Only owner can call this method")]
    fn non_owner_cannot_unpause() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

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

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

        set_context(owner);

        contract.pause();
        contract.assert_not_paused();
    }

    #[test]
    fn create_table_with_valid_buy_in_succeeds() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        contract.create_table(ONE_NEAR / 2);
    }

    #[test]
    #[should_panic(expected = "Buy-in is above the maximum allowed")]
    fn create_table_with_invalid_high_buy_in_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        contract.create_table(ONE_NEAR * 20);
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit to cover storage")]
    fn create_table_without_deposit_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context(alice);

        contract.create_table(ONE_NEAR * 2);
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn create_table_fails_when_paused() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

        set_context(owner);

        contract.pause();

        set_context_with_deposit(alice, ONE_NEAR);

        contract.create_table(ONE_NEAR * 2);
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit to cover storage")]
    fn create_table_with_insufficient_storage_deposit_fails() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, 1);

        contract.create_table(ONE_NEAR * 2);
    }

    #[test]
    fn create_table_with_excess_storage_deposit_succeeds() {
        let owner = account("owner.testnet");
        let alice = account("alice.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

        set_context_with_deposit(alice, ONE_NEAR * 3);

        contract.join_table(table_id);
    }

    #[test]
    #[should_panic(expected = "Table does not exist")]
    fn cannot_join_nonexistent_table() {
        let owner = account("owner.testnet");
        let bob = account("bob.testnet");

        set_context(owner.clone());

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner.clone(), ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice, ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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

        let mut contract = Contract::new(owner, ONE_NEAR, ONE_NEAR * 10);

        set_context_with_deposit(alice.clone(), ONE_NEAR);

        let table_id = contract.create_table(ONE_NEAR * 2);

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
    fn game_starts_with_player_internal_balances() {
        let (contract, table_id, alice, bob) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.player_balances.len(), 2);
        assert_eq!(get_player_balance(&table, &alice), ONE_NEAR * 2);
        assert_eq!(get_player_balance(&table, &bob), ONE_NEAR * 2);
    }

    #[test]
    fn game_starts_with_zero_pot() {
        let (contract, table_id, _, _) = setup_active_table();

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, 0);
    }

    #[test]
    fn raise_decreases_current_player_balance() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: ONE_NEAR / 2,
            },
        );

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(
            get_player_balance(&table, &alice),
            ONE_NEAR * 2 - ONE_NEAR / 2
        );
    }

    #[test]
    fn raise_increases_pot() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: ONE_NEAR / 2,
            },
        );

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, ONE_NEAR / 2);
    }

    #[test]
    #[should_panic(expected = "Raise amount exceeds player balance")]
    fn raise_larger_than_balance_fails() {
        let (mut contract, table_id, alice, _) = setup_active_table();

        set_context(alice);

        contract.submit_action(
            table_id,
            PlayerAction::Raise {
                amount: ONE_NEAR * 3,
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
                amount: 0,
            },
        );
    }

    #[test]
    fn check_does_not_change_balance_or_pot() {
        let (mut contract, table_id, alice, bob) = setup_active_table();

        set_context(alice.clone());

        contract.submit_action(table_id, PlayerAction::Check);

        let table = contract.get_table(table_id).unwrap();

        assert_eq!(table.pot, 0);
        assert_eq!(get_player_balance(&table, &alice), ONE_NEAR * 2);
        assert_eq!(get_player_balance(&table, &bob), ONE_NEAR * 2);
    }
}
