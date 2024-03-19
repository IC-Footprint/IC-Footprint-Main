use std::{cell::RefCell, collections::HashSet};

use candid::Principal;
use ic_cdk::{export_candid, query, update};
use ic_cdk::api::management_canister::http_request::{  
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,  
    TransformContext,  
   };
use ic_cdk::caller;
use ic_cdk::api::management_canister::http_request::TransformFunc;
use ic_cdk::api::call::call;
use candid::{CandidType, Deserialize};
use std::time::Duration;



struct Node {
    node_id: String,
    emmisions: u64,
}

struct Subnet {
    nodes: Vec<Node>,
}

// set api key
thread_local! {  
    static API_KEY: RefCell<String> = RefCell::new(String::new());
}

thread_local! {
    static AUTHORIZED_PRINCIPALS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
}

#[update]
pub fn set_api_key(api_key: String) {
    let caller_principal = caller();

    AUTHORIZED_PRINCIPALS.with(|p| {
        let authorized_principals = p.borrow();

        if authorized_principals.is_empty() || authorized_principals.contains(&caller_principal) {
            API_KEY.with(|k| *k.borrow_mut() = api_key);
        } else {
            ic_cdk::trap("Unauthorized: the caller is not allowed to set the API key.");
        }
    });
}

#[update]
pub fn authorize(principal: Principal) {
    let caller_principal = caller();

    AUTHORIZED_PRINCIPALS.with(|p| {
        let authorized_principals = p.borrow();

        if authorized_principals.is_empty() || authorized_principals.contains(&caller_principal) {
            AUTHORIZED_PRINCIPALS.with(|p| p.borrow_mut().insert(principal));
        } else {
            ic_cdk::trap("Unauthorized: the caller is not allowed to set the API key.");
        }
    });
}


#[update]
pub fn deauthorize(principal: Principal) {
    let caller_principal = caller();

    AUTHORIZED_PRINCIPALS.with(|p| {
        let authorized_principals = p.borrow();

        if authorized_principals.is_empty() || authorized_principals.contains(&caller_principal) {
            AUTHORIZED_PRINCIPALS.with(|p| p.borrow_mut().remove(&principal));
        } else {
            ic_cdk::trap("Unauthorized: the caller is not allowed to set the API key.");
        }
    });
}

#[query]
fn transform(raw: TransformArgs) -> HttpResponse {
    let headers = vec![HttpHeader {
        name: "Content-Type".to_string(),
        value: "application/json; charset=utf-8".to_string(),
    }];

    let mut res = HttpResponse {
        status: raw.response.status.clone(),
        body: raw.response.body.clone(),
        headers,
        ..Default::default()
    };

    if res.status == 200 {
        res.body = raw.response.body;
    } else {
        ic_cdk::api::print(format!("Received an error from coinbase: err = {:?}", raw));
    }
    res
}

// query api to get all nodes plus their emissions
#[update]
async fn get_emissions() -> String {
    let api_key = API_KEY.with(|k| k.borrow().clone());
    let url = "https://dashboard-backend.fly.dev/nodes/getNodeEmissions";

    let request = CanisterHttpRequestArgument {  
        url: url.to_string(),  
        method: HttpMethod::GET,  
        body: None,   
        max_response_bytes: None,  
        transform: Some(TransformContext {  
        function: TransformFunc(candid::Func {  
        principal: ic_cdk::api::id(),  
        method: "transform".to_string(),  
        }),  
        context: vec![],  
        }),  
        headers: vec![
            HttpHeader {
                name: "api-key".to_string(),
                value: format!("{}", api_key),
            },
            HttpHeader {
                name: "accept".to_string(),
                value: "application/json".to_string(),
            },
        ],  
       };

       // shedule self calling after 24 hours
    //    let delay = Duration::from_secs(24 * 60 * 60);
    //    let _ : Result<(), (ic_cdk::api::call::RejectionCode, String)> = call::<(), (), ()>(ic_cdk::api::id(), "get_emissions", (), delay).await;

       match http_request(request, 2_000_000_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            str_body
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            
            message
        }
    }
}
export_candid!();
