use near_sdk::json_types::{
    U128, 
};
use near_sdk::borsh::{
    self, 
    BorshDeserialize, 
    BorshSerialize
};
use near_sdk::{
    ext_contract,
    env,
    near_bindgen,
    Promise,
    init,
    AccountId
};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CLAIM_AMOUNT: u128 = 1_000_000_000_000_000_000_000_000;
const TRANSFER_GAS: u64 = 100_000_000_000_000;

#[ext_contract]
trait FaucetToken {
    fn transfer(&mut self, new_owner_id: String, amount: U128);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
struct Nep21Faucet {
    pub token_account_id: AccountId
}

impl Default for Nep21Faucet {
    fn default() -> Self{
        panic!("Contract needs to be initialized before usage")
    }
}

#[near_bindgen]
impl Nep21Faucet {
    #[init]
    pub fn init(token_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initted");
        Self {
            token_account_id
        }
    }

    /* TODO: Add some minimal sybil resistence where accounts can only claim every 30 minutes */
    pub fn claim(&self) -> Promise {
        return faucet_token::transfer(
            env::predecessor_account_id(), 
            U128(CLAIM_AMOUNT), 
            &self.token_account_id, 
            0, 
            TRANSFER_GAS
        );
    }

    pub fn get_token_account_id(&self) -> &AccountId {
        return &self.token_account_id;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
	use super::*;
	mod test_utils;
	use test_utils::{ExternalUser, init_markets_contract, ntoy, token_account_id, faucet_account_id};
    use near_sdk::MockedBlockchain;
    use near_sdk::{VMContext, testing_env};
	use near_runtime_standalone::{RuntimeStandalone};
    use near_primitives::transaction::{ExecutionStatus, ExecutionOutcome};

    fn init_runtime_env() -> (RuntimeStandalone, ExternalUser, Vec<ExternalUser>) {
		let (mut runtime, root) = init_markets_contract();


		let mut accounts: Vec<ExternalUser> = vec![];
		for acc_no in 0..2 {
			let acc = if let Ok(acc) =
				root.create_external(&mut runtime, format!("account_{}", acc_no), ntoy(100))
			{
				acc
			} else {
				break;
			};
			accounts.push(acc);
		}

		return (runtime, root, accounts);
    }
    
    #[test]
    fn test_mockchain() {
        let (runtime, root, accounts) = init_runtime_env();
        assert_eq!(root.get_account_id(), "flux-dev".to_string());
    }
    
    #[test]
    fn test_deposit_and_claim() {
        let (mut runtime, root, accounts) = init_runtime_env();
        root.transfer(&mut runtime, faucet_account_id(), U128(CLAIM_AMOUNT * 5));
        let faucet_contract_balance = root.get_balance(&mut runtime, faucet_account_id());
        assert_eq!(faucet_contract_balance, U128(CLAIM_AMOUNT * 5));
        
        accounts[0].claim(&mut runtime);
        
        let faucet_contract_balance = root.get_balance(&mut runtime, faucet_account_id());
        assert_eq!(faucet_contract_balance, U128(CLAIM_AMOUNT * 4));
        let claimer_balance = root.get_balance(&mut runtime, accounts[0].get_account_id());
        assert_eq!(claimer_balance, U128(CLAIM_AMOUNT));        
    }
}