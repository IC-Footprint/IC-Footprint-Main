use ic_cdk::{
    api::management_canister::http_request::{
        http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse,
        TransformArgs, TransformContext, TransformFunc,
    },
    query, update,
};
use serde_derive::{Deserialize, Serialize};
// use ic_cdk::caller;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::cell::RefCell;
use ic_cdk::api::caller;
use candid::Principal;
use std::collections::HashSet;
// use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
// use std::time::{SystemTime, UNIX_EPOCH};



#[derive(Serialize, Deserialize)]
struct Context {
    project_id: String,
    ticket_count: u64,
}

#[derive(Serialize, Deserialize)]
struct ContributionRequest {
    amount: u64,
    on_behalf_of: String,
    unit: String,
    currency: String,
    project: String,
}

#[derive(Serialize, Deserialize)]
struct EntityRequest {
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize)]
struct GetContributions {
    start_date: String,
    end_date: String,
    entity: String,
}

thread_local! {  
    static API_KEY: RefCell<String> = RefCell::new(String::new());
}

thread_local! {
    static AUTHORIZED_PRINCIPALS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::new());
}


static COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_uuid() -> String {
    let now = ic_cdk::api::time();
    let nanoseconds = now as u64;
    let counter_value = COUNTER.fetch_add(1, Ordering::SeqCst);
    let combined_data = nanoseconds ^ counter_value;
    let hex_string = format!("{:032x}", combined_data); 

    if hex_string.len() != 32 {
        panic!("Unexpected string length for UUID generation"); 
    }

    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex_string[..8],
        &hex_string[8..12],
        &hex_string[12..16],
        &hex_string[16..20],
        &hex_string[20..]
    );

    ic_cdk::println!("Generated UUID: {} .", uuid);

    uuid
}

#[update]
pub fn set_api_key(api_key: String) {{
    let caller_principal = caller();

    AUTHORIZED_PRINCIPALS.with(|p| {{
        let authorized_principals = p.borrow();

        if authorized_principals.is_empty() || authorized_principals.contains(&caller_principal) {{
            API_KEY.with(|k| *k.borrow_mut() = api_key);
        }} else {{
            ic_cdk::trap("Unauthorized: the caller is not allowed to set the API key.");
        }}
    }});
}}

#[update]
pub fn authorize(principal: Principal) {{
    AUTHORIZED_PRINCIPALS.with(|p| p.borrow_mut().insert(principal));
}}

#[update]
pub fn deauthorize(principal: Principal) {{
    AUTHORIZED_PRINCIPALS.with(|p| p.borrow_mut().remove(&principal));
}}



#[update]
pub async fn send(node_id: String, ticket_count: u64) -> String {
    let host = "api.dev.cawa.tech";
    let url = "https://api.dev.cawa.tech/api/v1/contribution";
    let project_id = "018aa416-3fab-46c1-b9c1-6fab067b70b7";
    let api_key = API_KEY.with(|k| k.borrow().clone());
   

    let idempotency_key = generate_uuid();
    let request_headers = vec![
        HttpHeader {
            name: "host".to_string(),
            value: host.to_string(),
        },
        HttpHeader {
            name: "X-Cawa-IdempotencyKey".to_string(),
            value: idempotency_key.to_string(),
        },
        HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        },
        HttpHeader {
            name: "Authorization".to_string(),
            value: format!("Bearer {}", api_key),
        }
    ];


    let request_body_json = ContributionRequest {
        amount: ticket_count,
        on_behalf_of: format!("{}@carboncrowd.io", node_id).to_string(),
        unit: "cents".to_string(),
        currency: "EUR".to_string(),
        project: project_id.to_string(),
    };

    let json_string = serde_json::to_string(&request_body_json).expect("Failed to serialize request body");
    let json_utf8: Vec<u8> = json_string.into_bytes();
    let request_body: Option<Vec<u8>> = Some(json_utf8);

    let context = Context {
        project_id: project_id.to_string(),
        ticket_count,
    };

    let request = CanisterHttpRequestArgument {
        url: url.to_string(),
        max_response_bytes: None,
        method: HttpMethod::POST,
        headers: request_headers,
        body: request_body,
        transform: Some(TransformContext {
            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "transform".to_string(),
            }),
            context: serde_json::to_vec(&context).unwrap(),
        }),
    };

    match http_request(request, 2_000_000_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", str_body));
            let result: String = format!(
                "{}. See more info of the request sent at: {}/inspect",
                str_body, url
            );
            result
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            message
        }
    }
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

// create a PUT request to update entity for node
// TO DO: figure out how to make PUT requests on ICP
#[update]
pub async fn create_entity(node_id: String) -> String {
    let host = "api.dev.cawa.tech";
    let url = "https://api.dev.cawa.tech/api/v1/entity";
    let api_key = API_KEY.with(|k| k.borrow().clone());

    let request_headers = vec![
        HttpHeader {
            name: "host".to_string(),
            value: host.to_string(),
        },
        HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        },
        HttpHeader {
            name: "Authorization".to_string(),
            value: format!("Bearer {}", api_key),
        }
    ];

    let request_body_json = EntityRequest {
        name: format!("cawa{}", node_id).to_string(),
        email: format!("cawa{}@carboncrowd.io", node_id).to_string(),
    };

    let json_string = serde_json::to_string(&request_body_json).expect("Failed to serialize request body");
    let json_utf8: Vec<u8> = json_string.into_bytes();
    let request_body: Option<Vec<u8>> = Some(json_utf8);


    // TODO: Figure out how to make PUT requests
    let request = CanisterHttpRequestArgument {
        url: url.to_string(),
        max_response_bytes: None,
        method: HttpMethod::POST,
        headers: request_headers,
        body: request_body,
        transform: None,
    };

    match http_request(request, 2_000_000_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", str_body));
            let result: String = format!(
                "{}. See more info of the request sent at: {}/inspect",
                str_body, url
            );
            result
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            //Return the error as a string and end the method
            message
        }
    }
}

#[update]
async fn get_contributions(start_date: String, end_date: String, node_id: String) -> String {
    let api_key = API_KEY.with(|k| k.borrow().clone());
    // make the request body do a daily fetch dynamically
    let request_body_json = GetContributions {
        start_date: start_date,
        end_date: end_date,
        entity: format!("{}", node_id).to_string(),
    };

    let json_string = serde_json::to_string(&request_body_json).expect("Failed to serialize request body");
    let json_utf8: Vec<u8> = json_string.into_bytes();
    let request_body: Option<Vec<u8>> = Some(json_utf8);
    let url = "https://api.dev.cawa.tech/api/v1/contribution";

    let request = CanisterHttpRequestArgument {  
        url: url.to_string(),  
        method: HttpMethod::GET,  
        body: request_body,   
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
                name: "Authorization".to_string(),
                value: format!("Bearer {}", api_key),
            }
        ],  
       };

       match http_request(request, 2_000_000_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            ic_cdk::api::print(format!("{:?}", str_body));
            let result: String = format!(
                "{}. See more info of the request sent at: {}/inspect",
                str_body, url
            );
            result
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");

            //Return the error as a string and end the method
            message
        }
    }
}
