use super::*;
use near_sdk::MockedBlockchain;
use near_sdk::{VMContext, testing_env};
use near_crypto::{InMemorySigner, KeyType, Signer};
use near_runtime_standalone::{init_runtime_and_signer, RuntimeStandalone};
use near_primitives::{
    account::{AccessKey},
    errors::{RuntimeError, TxExecutionError},
    hash::CryptoHash,
    transaction::{ExecutionOutcome, ExecutionStatus, Transaction},
    types::{AccountId, Balance},
};
use std::collections::{HashMap};

use near_sdk::serde_json;
use near_sdk::serde_json::json;
use near_sdk::json_types::{U128, U64};

const GAS_STANDARD: u64 = 300000000000000;

pub fn ntoy(near_amount: Balance) -> Balance {
    near_amount * 10u128.pow(24)
}

pub fn faucet_account_id() -> String {
	return "fun_token_faucet".to_string();
}

pub fn token_account_id() -> String {
	return "token_account_id".to_string();
}

type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;

lazy_static::lazy_static! {
    static ref FAUCET_BYTES: &'static [u8] = include_bytes!("../../res/nep21_faucet.wasm").as_ref();
    static ref FUNGIBLE_TOKEN_BYTES: &'static [u8] = include_bytes!("../../res/fungible_token.wasm").as_ref();
}

fn outcome_into_result(outcome: ExecutionOutcome) -> TxResult {
    match outcome.status {
        ExecutionStatus::SuccessValue(_) => Ok(outcome),
        ExecutionStatus::Failure(_) => Err(outcome),
        ExecutionStatus::SuccessReceiptId(_) => panic!("Unresolved ExecutionOutcome run runtime.resolve(tx) to resolve the final outcome of tx"),
        ExecutionStatus::Unknown => unreachable!()
    }
}

pub struct ExternalUser {
    account_id: AccountId,
    signer: InMemorySigner,
}

impl ExternalUser {
	pub fn new(account_id: AccountId, signer: InMemorySigner) -> Self {
        Self { account_id, signer }
    }

    pub fn get_account_id(&self) -> AccountId {
        return self.account_id.to_string();
    }

    pub fn deploy_faucet(&self, runtime: &mut RuntimeStandalone) -> TxResult {
        let args = json!({
            "token_account_id": token_account_id(),
        }).to_string().as_bytes().to_vec();

        let tx = self
        .new_tx(runtime, faucet_account_id())
        .create_account()
        .transfer(99994508400000000000000000)
        .deploy_contract(FAUCET_BYTES.to_vec())
        .function_call("init".into(), args, GAS_STANDARD, 0)
        .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        runtime.process_all().unwrap();
        let ans = outcome_into_result(res);
        return ans;
    }

    pub fn deploy_token(&self, runtime: &mut RuntimeStandalone, owner_account_id: String, total_supply: U128) -> TxResult {
        // let args = json!({}).to_string().as_bytes().to_vec();
        let args = json!({
            "owner_id": owner_account_id,
            "total_supply": total_supply
        }).to_string().as_bytes().to_vec();

        let tx = self
        .new_tx(runtime, token_account_id())
        .create_account()
        .transfer(99994508400000000000000000)
        .deploy_contract(FUNGIBLE_TOKEN_BYTES.to_vec())
        .function_call("new".into(), args, GAS_STANDARD, 0)
        .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        runtime.process_all().unwrap();
        let ans = outcome_into_result(res);
        return ans;
    }

    fn new_tx(&self, runtime: &RuntimeStandalone, receiver_id: AccountId) -> Transaction {
        let nonce = runtime
        .view_access_key(&self.account_id, &self.signer.public_key())
        .unwrap()
        .nonce
        + 1;
        Transaction::new(
            self.account_id.clone(),
            self.signer.public_key(),
            receiver_id,
            nonce,
            CryptoHash::default(),
        )
    }

	pub fn create_external(
        &self,
        runtime: &mut RuntimeStandalone,
        new_account_id: AccountId,
        amount: Balance,
    ) -> Result<ExternalUser, ExecutionOutcome> {
        let new_signer = InMemorySigner::from_seed(&new_account_id, KeyType::ED25519, &new_account_id);
        let tx = self
        .new_tx(runtime, new_account_id.clone())
        .create_account()
        .add_key(new_signer.public_key(), AccessKey::full_access())
        .transfer(amount)
        .sign(&self.signer);

        let res = runtime.resolve_tx(tx);

        // TODO: this temporary hack, must be rewritten
        if let Err(err) = res.clone() {
            if let RuntimeError::InvalidTxError(tx_err) = err {
                let mut out = ExecutionOutcome::default();
                out.status = ExecutionStatus::Failure(TxExecutionError::InvalidTxError(tx_err));
                return Err(out);
            } else {
                unreachable!();
            }
        } else {
            outcome_into_result(res.unwrap())?;
            runtime.process_all().unwrap();
            Ok(ExternalUser {
                account_id: new_account_id,
                signer: new_signer,
            })
        }
	}

	pub fn claim(
        &self,
        runtime: &mut RuntimeStandalone,
    ) -> TxResult {
        let args = json!({})
        .to_string()
        .as_bytes()
        .to_vec();
        let tx = self
        .new_tx(runtime, faucet_account_id())
        .function_call("claim".into(), args, GAS_STANDARD, 0)
        .sign(&self.signer);
        let res = runtime.resolve_tx(tx).expect("resolving tx failed");
        runtime.process_all().expect("processing tx failed");
        let ans = outcome_into_result(res);
        return ans;
    }

    pub fn transfer(
        &self,
        runtime: &mut RuntimeStandalone,
        new_owner_id: AccountId,
        amount: U128
    ) -> TxResult {
        let args = json!({
            "new_owner_id": new_owner_id,
            "amount": amount,
        })
        .to_string()
        .as_bytes()
        .to_vec();
        let tx = self
        .new_tx(runtime, token_account_id())
        .function_call("transfer".into(), args, GAS_STANDARD, 0)
        .sign(&self.signer);
        let res = runtime.resolve_tx(tx).expect("resolving tx failed");
        runtime.process_all().expect("processing tx failed");
        let ans = outcome_into_result(res);
        return ans;
    }

    pub fn get_token_account_id(
        &self,
        runtime: &mut RuntimeStandalone
    ) -> U128 {
        let market_price_json = runtime
        .view_method_call(
            &(faucet_account_id()),
            "get_token_account_id",
            json!({})
            .to_string()
            .as_bytes(),
        )
        .unwrap()
        .0;

        let data: serde_json::Value = serde_json::from_slice(market_price_json.as_slice()).unwrap();
        let res = serde_json::from_value(serde_json::to_value(data).unwrap()).unwrap();

        return res;
    }
    
    pub fn get_balance(
        &self,
        runtime: &mut RuntimeStandalone,
        account_id: String,
    ) -> U128 {
        let market_price_json = runtime
        .view_method_call(
            &(token_account_id()),
            "get_balance",
            json!({
                "owner_id": account_id
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap()
        .0;

        let data: serde_json::Value = serde_json::from_slice(market_price_json.as_slice()).unwrap();
        let res = serde_json::from_value(serde_json::to_value(data).unwrap()).unwrap();

        return res;
    }
}

pub fn init_markets_contract() -> (RuntimeStandalone, ExternalUser) {
    let (mut runtime, signer) = init_runtime_and_signer(&"flux-dev".into());
    let root = ExternalUser::new("flux-dev".into(), signer);

    root.deploy_faucet(&mut runtime).unwrap();
    root.deploy_token(&mut runtime, "flux-dev".to_string(), U128(100000000000000000000000000000000)).unwrap();
    
    return (runtime, root);
}