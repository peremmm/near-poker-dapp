use near_sdk::{env, near, AccountId, PanicOnDefault};

pub type Balance = u128;

#[near(serializers = [json])]
pub struct BuyInRangeView {
    pub min_buy_in: Balance,
    pub max_buy_in: Balance,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    min_buy_in: Balance,
    max_buy_in: Balance,
    paused: bool,
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

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Only owner can call this method"
        );
    }
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
}
