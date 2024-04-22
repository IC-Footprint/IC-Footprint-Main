use ic_cdk::{
    api::management_canister::http_request::{
        http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse,
        TransformArgs, TransformContext, TransformFunc,
    },
    query, update,
};
use serde_derive::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::cell::RefCell;
use ic_cdk::api::caller;
use candid::Principal;
use std::collections::HashSet;





#[derive(Serialize, Deserialize)]
struct Context {
    project_id: String,
    ticket_count: f64,
}

#[derive(Serialize, Deserialize)]
struct ContributionRequest {
    amount: u64,
    on_behalf_of: String,
    unit: String,
    currency: String,
    project: String,
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
        let mut authorized_principals = p.borrow_mut();
    
        if authorized_principals.is_empty() || authorized_principals.contains(&caller_principal) {
            authorized_principals.insert(principal);
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


#[update]
pub async fn send(client: String, ticket_count: f64) -> String {

    // check if the caller is authorized
    let caller = caller(); 
    let is_authorized = AUTHORIZED_PRINCIPALS.with(|p| {
        let authorized_principals = p.borrow();
        authorized_principals.is_empty() || authorized_principals.contains(&caller)
    });

    if !is_authorized {
        return serde_json::to_string(&json!({"error": "Unauthorized: the caller is not allowed to perform this action."})).unwrap();
    }
    
    let host = "api.cawa.tech";
    let url = "https://api.cawa.tech/api/v1/contribution/prepaid";
    let project_id = "018828f6-8718-4550-9c6e-83a0fa52402d";
    let api_key = API_KEY.with(|k| k.borrow().clone());
    
    let ticket_count_u64 = ticket_count as u64;
   

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
        amount: ticket_count_u64,
        on_behalf_of: format!("cawa+{}@carboncrowd.io", client).to_string(),
        unit: "kilos".to_string(),
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

    match http_request(request, 21_000_000_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
            .expect("Transformed response is not UTF-8 encoded.");

            ic_cdk::api::print(format!("Response from cawa: {}", str_body));
            
        // Check if the response status code indicates an error
        if response.status >= 400u32 && response.status < 600u32 {
            // Parse the error message from the response body
            let parsed: serde_json::Value = match serde_json::from_str(&str_body) {
                Ok(value) => value,
                Err(e) => ic_cdk::trap(&format!("Failed to parse error response as JSON: {:?}", e)),
            };
            let error_message = parsed["error"].as_str().unwrap_or("Unknown error");
            ic_cdk::trap(&format!("CAWA API error: {:?}", error_message));
        }
        
        // Parse the JSON response
        let parsed: serde_json::Value = serde_json::from_str(&str_body)
            .expect("JSON was not well-formatted");

        // Extract the id field
        let id_array = parsed["id"].as_array()
            .expect("id field not found or not an array");

        // Get the first element of the array
        let id = id_array.get(0)
            .and_then(|v| v.as_str())
            .expect("id array is empty or contains non-string");

        id.to_string()
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


#[update]
pub async fn get_contributions() -> String {
    let api_key = API_KEY.with(|k| k.borrow().clone());
    let url = "https://api.cawa.tech/api/v1/contribution";

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
                name: "Authorization".to_string(),
                value: format!("Bearer {}", api_key),
            }
        ],  
       };

       match http_request(request, 21_000_000_000).await {
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


#[update]
pub async fn get_contribution_by_entity(node_id: String) -> String {
   let api_key = API_KEY.with(|k| k.borrow().clone());
   let url = format!("https://api.cawa.tech/api/v1/contribution?entity=cawa%2B{}@carboncrowd.io",node_id);

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
                name: "Authorization".to_string(),
                value: format!("Bearer {}", api_key),
          }
     ],  
    };

    match http_request(request, 21_000_000_000).await {
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

// get contribution by id
#[update]
pub async fn get_contribution_by_id(contribution_id: String) -> String {
    let api_key = API_KEY.with(|k| k.borrow().clone());
    let url = format!("https://api.cawa.tech/api/v1/contribution?id={}",contribution_id);

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
                name: "Authorization".to_string(),
                value: format!("Bearer {}", api_key),
          }
     ],  
    };

    match http_request(request, 21_000_000_000).await {
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


