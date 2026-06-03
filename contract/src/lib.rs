use near_sdk::{
    borsh, env, near, AccountId, BorshStorageKey, NearToken, PanicOnDefault, Promise,
};
use near_sdk::store::UnorderedMap;

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

#[derive(Clone)]
#[near(serializers = [borsh])]
pub struct Table {
    pub id: u64,
    pub creator_id: AccountId,
    pub buy_in: Balance,
    pub players: Vec<AccountId>,
    pub status: TableStatus,
    pub created_at: u64,
}

#[near(serializers = [json])]
pub struct TableView {
    pub id: u64,
    pub creator_id: AccountId,
    pub buy_in: Balance,
    pub players: Vec<AccountId>,
    pub status: TableStatus,
    pub created_at: u64,
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
            table.status = TableStatus::Active;
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

    pub fn get_table(&self, table_id: u64) -> Option<TableView> {
        self.tables.get(&table_id).map(|table| TableView {
            id: table.id,
            creator_id: table.creator_id.clone(),
            buy_in: table.buy_in,
            players: table.players.clone(),
            status: table.status.clone(),
            created_at: table.created_at,
        })
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
}
