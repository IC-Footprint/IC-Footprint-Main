use std::{cell::RefCell, collections::HashSet};

use candid::Principal;
use ic_cdk::{export_candid, query, update};
use ic_cdk::api::management_canister::http_request::{  
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,  
    TransformContext,  
   };
use ic_cdk::caller;
use ic_cdk::api::management_canister::http_request::TransformFunc;
// use ic_cdk::api::call::call;
use candid::{CandidType, Deserialize};
// use std::time::Duration;

#[derive(CandidType, Deserialize)]
struct Node {
    name: String,
    totalEmissions: f64,
    offsetEmissions: f64,
}


#[derive(CandidType, Deserialize)]
struct Client {
    client: String,
    nodes: Vec<Node>,
}

#[derive(CandidType, Deserialize)]
struct SimpleClient {
    name: String,
    node_ids: Vec<String>,
}

#[derive(CandidType, Deserialize)]
struct Payment {
    pub block_height: u64,
    pub payer: String,
    pub ticket_count: u64,
    pub ticket_price: u64,
    pub contribution_id: String,
}

// set api key
thread_local! {  
    static API_KEY: RefCell<String> = RefCell::new(String::new());
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
async fn get_emissions() -> Result<Vec<Node>, String> {
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

    match http_request(request, 21_000_000_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
        .expect("Transformed response is not UTF-8 encoded.");
        let json: serde_json::Value = serde_json::from_str(&str_body)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let nodes: Vec<Node> = json.as_array().unwrap().iter().map(|node| {
        let name = node["name"].as_str().unwrap().to_string();
        let totalEmissions = node["totalEmissions"].as_f64().unwrap();
        Node {
            name,
            totalEmissions,
            offsetEmissions: 0.0,
        }
    }).collect();
    Ok(nodes)
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {:?}, Error: {}", r, m);
            Err(message)
        }
    }
}

// offset emissions from nodes based on a client
#[update]
async fn offset_emissions(mut client: Client, mut offset: f64, node_name: Option<String>) -> String {
    // If offset is 0, return early.
    if offset == 0.0 {
        return "No emissions offset because offset amount is 0".to_string();
    }
    
    let mut offset_messages = Vec::new();

    if let Some(name) = node_name {
        // The client specified a node_name.
        if let Some(node) = client.nodes.iter_mut().find(|n| n.name == name) {
            // Found the node, offset the emissions.
            let mut offset_for_this_node = offset.min(node.totalEmissions);
            node.totalEmissions -= offset_for_this_node;
            node.offsetEmissions += offset_for_this_node;
            offset_messages.push(format!("Node {}: offset {} emissions", name, offset_for_this_node));
        }
    } else {
        // The client didn't specify a node_name.
        if !client.nodes.is_empty() {
            // The client is attached to some nodes, offset the emissions.
            for node in &mut client.nodes {
                let offset_for_this_node = offset.min(node.totalEmissions);
                node.totalEmissions -= offset_for_this_node;
                node.offsetEmissions += offset_for_this_node;
                offset_messages.push(format!("Node {}: offset {} emissions", node.name, offset_for_this_node));
            }
        } else {
            // The client isn't attached to any nodes, select a random set of nodes and offset the emissions.
            let nodes_future = select_random_nodes(); 
            let nodes = nodes_future.await;
            for mut node in nodes {
                let offset_for_this_node = offset.min(node.totalEmissions);
                node.totalEmissions -= offset_for_this_node;
                node.offsetEmissions += offset_for_this_node;
                offset_messages.push(format!("Node {}: offset {} emissions", node.name, offset_for_this_node));
            }
        }
    }

    offset_messages.join("\n")
}

#[update]
fn offset_from_nodes(mut nodes: Vec<Node>, mut offset: f64) {
    // Sort the nodes from highest to lowest totalEmissions.
    nodes.sort_by(|a, b| b.totalEmissions.partial_cmp(&a.totalEmissions).unwrap());

    for mut node in nodes {
        if offset <= 0.0 {
            break;
        }

        let offset_for_this_node = offset.min(node.totalEmissions);
        node.totalEmissions -= offset_for_this_node;
        node.offsetEmissions += offset_for_this_node;
        offset -= offset_for_this_node;
    }
}

#[update]
async fn select_random_nodes() -> Vec<Node> {
    let emissions_result = get_emissions().await;
    
    let mut nodes: Vec<Node> = match emissions_result {
        Ok(emissions) => emissions.into_iter().filter(|node| node.totalEmissions > 0.0).collect(),
        Err(_) => vec![],  // Handle the error appropriately.
    };
    
    // Sort the nodes from highest to lowest totalEmissions.
    nodes.sort_by(|a, b| b.totalEmissions.partial_cmp(&a.totalEmissions).unwrap());
    
    nodes
}

#[update]
async fn get_offset_emissions(simple_client: SimpleClient, payment: Vec<Payment>, node_name: Option<String>) -> String {
    let all_nodes_result = get_emissions().await;
    
    match all_nodes_result {
        Ok(all_nodes) => {
            let node_ids = simple_client.node_ids.clone();
            let nodes: Vec<Node> = all_nodes.into_iter().filter(|node| node_ids.contains(&node.name)).collect();
            let client = Client {
                client: simple_client.name,
                nodes,
            };
            let offset = payment.iter().fold(0.0, |acc, p| acc);
            offset_emissions(client, offset, node_name).await
        },
        Err(e) => {
            format!("Error getting emissions: {}", e)
        }
    }
}

export_candid!();
