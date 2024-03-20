use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use candid::{CandidType, Nat, Principal};

use ic_cdk::api::management_canister::http_request::HttpResponse;
use ic_cdk::api::management_canister::http_request::TransformArgs;

use ic_cdk::{
    api::call, caller, export_candid, id, init, post_upgrade, pre_upgrade, query, storage, update,
};
use icrc_ledger_types::{
    icrc1::account::Account,
    icrc2::transfer_from::{TransferFromArgs, TransferFromError},
};
use serde_derive::{Deserialize, Serialize};
use crate::cawa_poster::send;

type PaymentStore = BTreeMap<u64, Payment>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Hash, PartialEq)]
pub struct Conf {
    ledger_canister_id: Principal,
    node_id: String,
    ticket_price: u64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
struct Payment {
    pub block_height: Nat,
    pub payer: String,
    pub ticket_count: u64,
    pub ticket_price: u64,
    pub contribution_id: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct Client {
    pub client: String,
    pub nodes: Option<Vec<String>>,
}

thread_local! {
    static PAYMENT_STORE: RefCell<PaymentStore> = RefCell::default();
    static TICKET_PRICE: Cell<u64> = Cell::new(0);
    static LEDGER_CANISTER_ID: RefCell<String> = RefCell::new(String::default());
    static NODE_ID: RefCell<String> = RefCell::new(String::default());
    static CURRENT_PAYMENT_ID: Cell<u64> = Cell::new(0);
    static CLIENT_STORE: RefCell<BTreeMap<String, Client>> = RefCell::default();
}

#[init]
fn init(conf: Conf) {
    TICKET_PRICE.set(conf.ticket_price);
    LEDGER_CANISTER_ID.set(conf.ledger_canister_id.to_string());

    // TODO: Change NODE_ID to either Client or Wallet ID
    NODE_ID.set(conf.node_id.to_string());
}

#[query(name = "getTicketPrice")]
fn get_ticket_price() -> u64 {
    return TICKET_PRICE.get();
}

#[query(name = "getPrice")]
fn get_price(ticket_count: u64) -> Nat {
    return Nat::from(ticket_count * TICKET_PRICE.get());
}

#[query(name = "getPurchases")]
fn get_purchases() -> Vec<Payment> {
    return PAYMENT_STORE.take().values().cloned().collect();
}

#[update(name = "registerPayment")]
async fn register_payment(ticket_count: u64) -> Result<u64, String> {
    let max_ticket_count = 1000000;
    let total_price = get_price(ticket_count);
    let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
    let node_id = NODE_ID.with(|id| id.borrow().clone());
    let contribution_id = send(node_id, ticket_count).await;

    if ticket_count <= 0 {
        return Err("Invalid ticket count".to_string());
    }

    if ticket_count > max_ticket_count {
        return Err("Ticket count is too big".to_string());
    }

    match Principal::from_text(ledger_canister_id) {
        Ok(principal) => {
            let transfer_args = TransferFromArgs {
                spender_subaccount: None,
                from: Account {
                    owner: caller(),
                    subaccount: None,
                },
                to: Account {
                    owner: id(),
                    subaccount: None,
                },
                amount: Nat::from(total_price),
                fee: None,
                memo: None,
                created_at_time: None,
            };

            let transfer_result = call::call::<
                (TransferFromArgs,),
                (Result<Nat, TransferFromError>,),
            >(principal, "icrc2_transfer_from", (transfer_args,))
            .await;

            if let Err(error) = transfer_result {
                ic_cdk::println!("Transfer error {:?} and message {}", error.0, error.1);
                return Err("Transaction Error".to_string());
            } else if let Ok((transactions_response,)) = transfer_result {
                match transactions_response {
                    Ok(block_height) => {
                        CURRENT_PAYMENT_ID.set(CURRENT_PAYMENT_ID.get() + 1);

                        PAYMENT_STORE.with(|store| {
                            store.borrow_mut().insert(
                                CURRENT_PAYMENT_ID.get(),
                                Payment {
                                    block_height,
                                    ticket_count,
                                    payer: caller().to_string(),
                                    ticket_price: TICKET_PRICE.get(),
                                    contribution_id,
                                },
                            )
                        });

                        return Ok(CURRENT_PAYMENT_ID.get());
                    }
                    Err(err) => {
                        ic_cdk::println!("Transfer error {:?}", err);
                        return Err("smth wrong".to_string());
                    }
                }
            } else {
                return Err("Unknown error".to_string());
            }
        }
        Err(err) => return Err(err.to_string()),
    }
}

#[pre_upgrade]
fn pre_upgrade() {
    PAYMENT_STORE.with(|payments| {
        storage::stable_save((
            payments,
            LEDGER_CANISTER_ID.take(),
            TICKET_PRICE.get(),
            CURRENT_PAYMENT_ID.get(),
            NODE_ID.take(),
        ))
        .unwrap()
    })
}

#[post_upgrade]
fn post_upgrade() {
    let (old_payments, ledger_canister_id, ticket_price, current_payment_id, node_id): (
        BTreeMap<u64, Payment>,
        String,
        u64,
        u64,
        String,
    ) = storage::stable_restore().unwrap();
    PAYMENT_STORE.with(|payments| *payments.borrow_mut() = old_payments);
    TICKET_PRICE.set(ticket_price);
    LEDGER_CANISTER_ID.set(ledger_canister_id);
    CURRENT_PAYMENT_ID.set(current_payment_id);
    NODE_ID.set(node_id);
}

#[update(name = "addClient")]
fn add_client(client: String, nodes: Option<Vec<String>>) {
    CLIENT_STORE.with(|store| {
        store.borrow_mut().insert(
            client.clone(),
            Client {
                client,
                nodes: Some(nodes),
            },
        )
    });
}

#[update(name = "removeClient")]
fn remove_client(client: String) {
    CLIENT_STORE.with(|store| {
        store.borrow_mut().remove(&client);
    });
}

#[query(name = "getClients")]
fn get_clients() -> Vec<Client> {
    return CLIENT_STORE.with(|store| store.borrow().values().cloned().collect());
}

export_candid!();
