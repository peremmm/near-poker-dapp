use near_sdk::near;

#[near(contract_state)]
pub struct Contract {
    placeholder: bool,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            placeholder: true,
        }
    }
}

#[near]
impl Contract {
    pub fn is_initialized(&self) -> bool {
        self.placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_test() {
        let contract = Contract::default();

        assert_eq!(contract.is_initialized(), true);
    }
}