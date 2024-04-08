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
use crate::cawa_poster::get_contribution_by_id;
use std::collections::HashSet;
use lazy_static::lazy_static;
use serde_json::json;
use serde_json::Value;

type PaymentStore = BTreeMap<u64, Payment>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Conf {
    ledger_canister_id: Principal,
    ticket_price: f64,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
struct Payment {
    pub block_height: Nat,
    pub payer: String,
    pub ticket_count: f64,
    pub ticket_price: f64,
    pub node_id: Option<String>,
    pub cawa_url: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Ord, PartialEq, Eq, PartialOrd)]
struct Client {
    pub name: String,
    pub node_ids: Vec<String>,
}

// define a list of nodes for a static client
lazy_static! {
    static ref NODES: Vec<String> = vec![
    "2lhun-vmepu-zc7ic-6lrjj-o4yda-nlmwn-m4i2i-h4oby-hwv4y-mpv7c-fae".to_string(),
    "2vmqi-lwsmy-rad75-wus6x-3rlbf-cgig6-rtheo-t42r2-i4vdg-jyguw-uqe".to_string(),
    "7qvtt-6lr4l-gx337-d6xvn-55ka2-6kcx2-c7iby-w36qm-u3ojf-ildyc-iqe".to_string(),
    "dkx5h-i5d56-xeb37-uldi5-dlgmp-atrom-gyfn7-edeih-i5ph6-2cquy-dqe".to_string(),
    "ebfjs-4yyde-6v3qo-zhvwl-2fvp2-4tarv-autre-tparv-kwi7f-t4wml-fae".to_string(),
    "ew5us-dwxat-fg6sp-opp2j-fjjg7-rcmyj-d3saj-vyych-f3keg-5kor5-yqe".to_string(),
    "ijqzn-wklsd-asfzv-ixif4-u7lrb-dblfi-mcq2d-nrwuk-d6cys-jgnar-7qe".to_string(),
    "mlbpy-pvy7z-gfwcb-5bakp-cuubm-prmi3-dppq6-rlepq-nkyvb-u7acf-fqe".to_string(),
    "nhkbm-fqj7r-jdjii-spvcr-to3iu-ccrkm-pils3-6io5l-ahtry-qhlly-4ae".to_string(),
    "rg3ny-2qkvz-btitk-sxuiu-fsl2h-nkmij-lagjs-y4bnq-cb63u-vh2rd-aqe".to_string(),
    "s5waw-pnvds-kpwqg-n6bly-r45vy-ax64x-sltbe-bbzl3-74npl-u4sos-mae".to_string(),
    "tliwu-apd5m-f5kxi-g37rt-cqcag-4kmua-v7dsq-xsmv4-pvmhr-la6hu-xae".to_string(),
    "wvxfb-eqeex-26z4r-vefff-llfh7-vv76x-pje4o-oh57m-eklny-zcs56-wae".to_string(),
    "6wnv3-oxkmg-rtcgq-slqsn-5xlwu-7a27t-524pa-st7b3-epsck-tixsa-uqe".to_string(),
    "asad5-qg3gv-p5hrf-7liwa-zkoia-t2imf-sztmb-okb3k-gc6ei-xzp77-5qe".to_string(),
    "b7ldm-xgdir-7eraz-sws5m-tzstj-clkrl-hs444-ty5nj-qnkrx-nofhs-hae".to_string(),
    "buqsd-72zlu-hbgr2-hzjnr-mgvyj-6ldtg-2hdsd-igtpb-q7p2l-4j43v-5ae".to_string(),
    "cttab-pom2f-5a3ni-6rxwe-yt2nc-4kn4f-3jhd3-wo6hd-576cr-y6nge-2ae".to_string(),
    "jq33i-hlo5d-hyou6-wsgu4-vi7o6-upgg3-pzawk-les4l-gn3fg-eplfx-eae".to_string(),
    "jux3z-ivwyz-ury64-jth4b-rrbfi-sx5af-ci72l-j4ot3-nl5jk-z4ilu-6qe".to_string(),
    "jys4w-lodfn-h5325-xters-z3hf2-v7imc-ow7dh-py22m-epjvt-aldwt-oqe".to_string(),
    "n74se-c345d-2jd4b-leuu6-jyvim-vcllk-aahj2-vjpvn-pbpks-xtgy2-4qe".to_string(),
    "oiso5-hxkkc-elqlo-iyoqg-7oux4-5mscl-zkvoh-b2432-ohrir-b5fcm-gae".to_string(),
    "ptzzn-jphjl-476ql-lgyrk-37oa6-zxhat-ynn4f-fm2ry-r3cmf-lx7e6-uae".to_string(),
    "ux7wu-iidyv-r5cth-tz6n5-4xryn-av37c-24lrk-ozfaq-7sjax-ohkd2-6ae".to_string(),
   " uznxh-cff3i-uso5i-u27p7-ardz7-4vihf-vkme6-i76b5-oejzx-2xewb-lqe".to_string(),
];
}
lazy_static! {
    static ref CLIENT: Client = Client {
        name: "OpenChat".to_string(),
        node_ids: NODES.clone(),
    };
}

thread_local! {
    static PAYMENT_STORE: RefCell<PaymentStore> = RefCell::default();
    static TICKET_PRICE: Cell<f64> = Cell::new(0.0);
    static LEDGER_CANISTER_ID: RefCell<String> = RefCell::new(String::default());
    static CURRENT_PAYMENT_ID: Cell<u64> = Cell::new(0);
    static CLIENT_STORE: RefCell<BTreeMap<String, Client>> = RefCell::default();
    static AUTHORIZED_PRINCIPALS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
}

#[init]
fn init(conf: Conf) {
    TICKET_PRICE.set(conf.ticket_price);
    LEDGER_CANISTER_ID.set(conf.ledger_canister_id.to_string());
}

#[query(name = "getTicketPrice")]
fn get_ticket_price() -> f64 {
    return TICKET_PRICE.get();
}

#[query(name = "getPrice")]
fn get_price(ticket_count: f64) -> f64 {
    return f64::from(ticket_count * TICKET_PRICE.get());
}

#[query(name = "getPurchases")]
fn get_purchases() -> Vec<Payment> {
    return PAYMENT_STORE.take().values().cloned().collect();
}

#[update(name = "registerPayment")]
async fn register_payment(ticket_count: u64, nodeId: Option<String>) -> String {
    let max_ticket_count = 1000000;
    let total_price = get_price(ticket_count as f64);
    let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
    let client1 = CLIENT.name.clone();

    if ticket_count <= 0 {
        return serde_json::to_string(&json!({"error": "Invalid ticket count"})).unwrap();
    }

    if ticket_count > max_ticket_count {
        return serde_json::to_string(&json!({"error": "Ticket count is too big"})).unwrap();
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
                amount: Nat::from(total_price as u64),
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
                return serde_json::to_string(&json!({"error": "Transaction Error"})).unwrap();
            } else if let Ok((transactions_response,)) = transfer_result {
                match transactions_response {
                    Ok(block_height) => {
                        CURRENT_PAYMENT_ID.set(CURRENT_PAYMENT_ID.get() + 1);

                       // Define contribution_id before the conditional logic
                       let mut contribution_id = String::new();

                        if let Some(ref node_id) = nodeId {
                            // check if the node_id is a specified string
                            if node_id == "eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe" {
                                // set client to Openchat
                                let client1 = CLIENT.name.clone();
                            }
                            else {
                                // set up a new client for nodes that are not Openchat
                                // check if the node_id is not in the list of node_ids for client Openchat
                                if !CLIENT.node_ids.contains(&node_id) {
                                // if not create a new client for these nodes
                                let client1 = Client {
                                    name: "bkyz2-fmaaa-aaaaa-qaaaq-cai".to_string(),
                                    node_ids: vec![node_id.to_string()],
                                };
                                contribution_id = send(client1.name.clone(), ticket_count as f64).await;
                            }
                        }
                        } else {
                            contribution_id = send(client1, ticket_count as f64).await;
                        }

                        let cawa_url = get_proof(contribution_id.clone()).await;
                        let payment = Payment {
                            block_height,
                            ticket_count: ticket_count as f64,
                            payer: caller().to_string(),
                            ticket_price: TICKET_PRICE.get(), 
                            node_id: nodeId.clone(),
                            cawa_url: cawa_url,
                        };

                        PAYMENT_STORE.with(|store| {
                            store.borrow_mut().insert(
                                CURRENT_PAYMENT_ID.get(),
                                payment.clone(),
                            )
                        });

                        // Return the payment struct as a JSON string
                        return serde_json::to_string(&payment).unwrap();
                    },
                    Err(e) => {
                        return serde_json::to_string(&json!({"error": format!("The http_request resulted into error. Error: {:?}", e)})).unwrap();
                    }
                }
            } else {
                return serde_json::to_string(&json!({"error": "Unknown error"})).unwrap();
            }
        }
        Err(err) => return serde_json::to_string(&json!({"error": err.to_string()})).unwrap(),
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
            CLIENT.name.clone(),
        ))
        .unwrap()
    })
}

#[post_upgrade]
fn post_upgrade() {
    let (old_payments, ledger_canister_id, ticket_price, current_payment_id, client): (
        BTreeMap<u64, Payment>,
        String,
        f64,
        u64,
        String,
    ) = storage::stable_restore().unwrap();
    PAYMENT_STORE.with(|payments| *payments.borrow_mut() = old_payments);
    TICKET_PRICE.set(ticket_price);
    LEDGER_CANISTER_ID.set(ledger_canister_id);
    CURRENT_PAYMENT_ID.set(current_payment_id);
    // NODE_ID.set(node_id);
    CLIENT.name.clone();
}

#[update(name = "setOffsetEmissions")]
async fn set_offset_emissions(nodeId: Option<String>) -> String {

    // make sure only authorized principals can call this function
    let caller_principal = caller();
    AUTHORIZED_PRINCIPALS.with(|p| {
        let authorized_principals = p.borrow();
        if !authorized_principals.contains(&caller_principal) {
            ic_cdk::trap("Unauthorized: the caller is not allowed to perform this action.");
        }
    });
    
    let canister_id = Principal::from_text("jhfj2-iqaaa-aaaak-qddxq-cai").expect("Failed to create Principal");
    let mut client = CLIENT.clone();
    let payment: Vec<_> = PAYMENT_STORE.with(|payments| payments.borrow().values().cloned().collect());
    
    if let Some(ref node_id) = nodeId {
        if !CLIENT.node_ids.contains(&node_id) {
            let client = Client {
                name: "bkyz2-fmaaa-aaaaa-qaaaq-cai".to_string(),
                node_ids: vec![node_id.to_string()],
        };
    }
    }
    match ic_cdk::api::call::call::<(Client, Vec<Payment>, Option<String>), (String,)>(canister_id, "get_offset_emissions", (client, payment, None)).await {
        Ok((response,)) => response,
        Err(e) => format!("Error: {:?}", e),
    }
}

#[query(name = "getPurchasesByNodeId")]
fn get_purchases_by_node_id(node_id: String) -> Vec<Payment> {{
    let node_id_clone = node_id.clone();
    PAYMENT_STORE.with(|store| {{
        store.borrow()
            .values()
            .cloned()
            .filter(|payment| payment.node_id.as_ref() == Some(&node_id_clone))
            .collect()
    }})
}}

#[update(name = "get_proof")]
pub async fn get_proof(contribution_id: String) -> String {
    let json = get_contribution_by_id(contribution_id).await;
    let data: Value = match serde_json::from_str(&json) {
        Ok(data) => data,
        Err(_) => return "Proof URL does not exist".to_string(),
    };
    if let Some(array) = data.as_array() {
        if !array.is_empty() {
            if let Some(proof) = array[0]["proof"].as_str() {
                return proof.to_string();
            }
        }
    }
    "Proof URL does not exist".to_string()
}

// method to withdraw funds from the canister to wallet
#[update(name = "withdraw")]
async fn withdraw(wallet: Principal, amount: u64) -> String {
    //check if caller is authorized
    let caller = caller(); 
    AUTHORIZED_PRINCIPALS.with(|p| {
        let authorized_principals = p.borrow();
        if !authorized_principals.contains(&caller) {
            ic_cdk::trap("Unauthorized: the caller is not allowed to withdraw payments.");
        }
    });

    let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
    match Principal::from_text(ledger_canister_id) {
        Ok(principal) => {
            let transfer_args = TransferFromArgs {
                spender_subaccount: None,
                from: Account {
                    owner: id(),
                    subaccount: None,
                },
                to: Account {
                    owner: wallet,
                    subaccount: None,
                },
                amount: Nat::from(amount),
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
                return serde_json::to_string(&json!({"error": "Transaction Error"})).unwrap();
            } else if let Ok((transactions_response,)) = transfer_result {
                match transactions_response {
                    Ok(block_height) => {
                        return serde_json::to_string(&json!({"block_height": block_height})).unwrap();
                    },
                    Err(e) => {
                        return serde_json::to_string(&json!({"error": format!("The http_request resulted into error. Error: {:?}", e)})).unwrap();
                    }
                }
            } else {
                return serde_json::to_string(&json!({"error": "Unknown error"})).unwrap();
            }
        }
        Err(err) => return serde_json::to_string(&json!({"error": err.to_string()})).unwrap(),
    }
}


export_candid!();
