use ic_cdk::{
    api::management_canister::http_request::{
        http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse,
        TransformArgs, TransformContext, TransformFunc,
    },
    query, update,
};
use serde_derive::{Deserialize, Serialize};
use ic_cdk::caller;

// const authorized_users = vec![]; 

// fn check_authorization(caller: &str, authorized_users: &Vec<&str>) -> bool {
//     authorized_users.contains(&caller)
// }



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

#[update]
pub async fn send(node_id: String, ticket_count: u64, api_key: String) -> String {
    let host = "api.dev.cawa.tech";
    let url = "https://api.dev.cawa.tech/api/v1/contribution";
    let project_id = "018aa416-3fab-46c1-b9c1-6fab067b70b7";

    // if !check_authorization(caller, &authorized_users) {
    //     return "Unauthorized".to_string();
    // }

   
    fn generate_uuid() -> String {
        let uuid = "00000000-0000-4000-8000-000000000000";
        return uuid.to_string();
    }

    let idempotency_key = generate_uuid();
    let request_headers = vec![
        HttpHeader {
            name: "Host".to_string(),
            value: format!("{host}:443"),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "carbon_canister".to_string(),
        },
        HttpHeader {
            name: "X-Cawa-IdempotencyKey".to_string(),
            value: idempotency_key,
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

            //Return the error as a string and end the method
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

// create an cawa entity for node
#[update]
pub async fn create_entity(node_id: String, api_key: String) -> String {
    let host = "api.dev.cawa.tech";
    let url = "https://api.dev.cawa.tech/api/v1/entity";

    // if !check_authorization(caller, &authorized_users) {
    //     return "Unauthorized".to_string();
    // }

    fn generate_uuid() -> String {
        let uuid = "00000000-0000-4000-8000-000000000001";
        return uuid.to_string();
    }

    let idempotency_key = generate_uuid();
    let request_headers = vec![
        HttpHeader {
            name: "Host".to_string(),
            value: format!("{host}:443"),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "carbon_canister".to_string(),
        },
        HttpHeader {
            name: "X-Cawa-IdempotencyKey".to_string(),
            value: idempotency_key,
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
        name: node_id.to_string(),
        email: format!("{}@carboncrowd.io", node_id).to_string(),
    };

    let json_string = serde_json::to_string(&request_body_json).expect("Failed to serialize request body");
    let json_utf8: Vec<u8> = json_string.into_bytes();
    let request_body: Option<Vec<u8>> = Some(json_utf8);

    // let context = Context {
    //     project_id: project_id.to_string(),
    //     ticket_count,
    // };

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
            context: vec![],
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

            //Return the error as a string and end the method
            message
        }
    }
}

