use std::{cell::RefCell, collections::BTreeMap};

use candid::Principal;
use ic_cdk::{export_candid, query, update};

type EscrowStore = BTreeMap<String, Principal>;

thread_local! {
  static ESCROW_STORE: RefCell<EscrowStore> = RefCell::default();
}

#[query(name = "getNodeEscrow")]
fn get_node_escrow(id: String) -> Result<Principal, ()> {
    match ESCROW_STORE.take().get(&id) {
        Some(principal) => Ok(*principal),
        None => Err(()),
    }
}

#[update(name = "addNodeEscrow")]
fn add_node_escrow(id: String, escrow: Principal) -> Result<(), ()> {
    match ESCROW_STORE.take().get(&id) {
        Some(_) => return Err(()),
        None => {
            ESCROW_STORE.with_borrow_mut(|store| store.insert(id, escrow));
            return Ok(());
        }
    }
}

export_candid!();
