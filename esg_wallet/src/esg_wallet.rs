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
use lazy_static::lazy_static;

type PaymentStore = BTreeMap<u64, Payment>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Hash, PartialEq)]
pub struct Conf {
    ledger_canister_id: Principal,
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
struct Payment {
    pub block_height: Nat,
    pub payer: String,
    pub ticket_count: u64,
    pub ticket_price: u64,
    pub contribution_id: String,
    pub node_id: Option<String>,
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
    static TICKET_PRICE: Cell<u64> = Cell::new(1);
    static LEDGER_CANISTER_ID: RefCell<String> = RefCell::new(String::default());
    static CURRENT_PAYMENT_ID: Cell<u64> = Cell::new(0);
    static CLIENT_STORE: RefCell<BTreeMap<String, Client>> = RefCell::default();
}

#[init]
fn init(conf: Conf) {
    // TICKET_PRICE.set(conf.ticket_price);
    LEDGER_CANISTER_ID.set(conf.ledger_canister_id.to_string());
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
async fn register_payment(ticket_count: u64, nodeId: Option<String>) -> String {
    let max_ticket_count = 1000000;
    let total_price = get_price(ticket_count);
    let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
    let client1 = CLIENT.name.clone();

    if ticket_count <= 0 {
        return "Invalid ticket count".to_string();
    }

    if ticket_count > max_ticket_count {
        return "Ticket count is too big".to_string();
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
                return "Transaction Error".to_string();
            } else if let Ok((transactions_response,)) = transfer_result {
                match transactions_response {
                    Ok(block_height) => {
                        CURRENT_PAYMENT_ID.set(CURRENT_PAYMENT_ID.get() + 1);

                        // check if node_id is procided
                        if let Some(ref node_id) = nodeId {
                            // check if the node_id is not already in the list
                            if !CLIENT.node_ids.contains(&node_id) {
                                // if not create a new client with the node_id
                                let client1 = Client {
                                    name: "bkyz2-fmaaa-aaaaa-qaaaq-cai".to_string(),
                                    node_ids: vec![node_id.to_string()],
                                };
                            }
                        }

                        let contribution_id = send(client1, ticket_count).await;

                        let payment = Payment {
                            block_height,
                            ticket_count,
                            payer: caller().to_string(),
                            ticket_price: TICKET_PRICE.get(),
                            contribution_id,
                            node_id: nodeId.clone(),
                        };

                        PAYMENT_STORE.with(|store| {
                            store.borrow_mut().insert(
                                CURRENT_PAYMENT_ID.get(),
                                payment.clone(),
                            )
                        });

                        let _ = set_offset_emissions(nodeId).await;
                        format!(
                            "Payment: block_height: {}, ticket_count: {}, payer: {}, ticket_price: {}, contribution_id: {}",
                            payment.block_height, payment.ticket_count, payment.payer, payment.ticket_price, payment.contribution_id
                        )
                    },
                    Err(e) => {
                        format!("The http_request resulted into error. Error: {:?}", e)
                    }
                }
            } else {
                return "Unknown error".to_string();
            }
        }
        Err(err) => return err.to_string(),
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
        u64,
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
fn get_purchases_by_node_id(node_id: String) -> Vec<Payment> {
    PAYMENT_STORE.with(|store| {
        store.borrow()
            .values()
            .cloned()
            .filter(|payment| payment.node_id.as_ref().map_or(false, |id| id == &node_id))
            .collect()
    })
}

export_candid!();
