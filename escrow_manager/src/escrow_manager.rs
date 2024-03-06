use std::{cell::RefCell, collections::BTreeMap};

use candid::Principal;
use ic_cdk::{export_candid, query, update};

type EscrowStore = BTreeMap<String, Principal>;

thread_local! {
  static ESCROW_STORE: RefCell<EscrowStore> = RefCell::default();
}

#[query(name = "getNodeEscrow")]
fn get_node_escrow(id: String) -> Result<Principal, ()> {
    let result = ESCROW_STORE.try_with(|store| -> Result<_, ()> {
        let store_ref = store.borrow();
        Ok(store_ref.get(&id).cloned().ok_or(())?)
    });

    result.unwrap_or(Err(()))
}

#[update(name = "addNodeEscrow")]
fn add_node_escrow(id: String, escrow: Principal) -> Result<(), ()> {
    let result = ESCROW_STORE.try_with(|store| -> Result<_, ()> {
        let mut store_ref = store.borrow_mut();
        if store_ref.contains_key(&id) {
            return Err(());
        }
        store_ref.insert(id, escrow);
        Ok(())
    });

    result.unwrap_or(Err(()))
}

export_candid!();
