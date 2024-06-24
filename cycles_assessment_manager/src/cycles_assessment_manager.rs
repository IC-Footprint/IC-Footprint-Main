use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::{query, update};
use std::cell::RefCell;

thread_local! {
    static PREV_CYCLES: RefCell<u64> = RefCell::new(0);
    static AVG_BURN_RATE: RefCell<f64> = RefCell::new(0.0);
    static DIFF: RefCell<Vec<f64>> = RefCell::new(Vec::new())
}

#[derive(CandidType, Deserialize, Debug)]
struct CanisterStatusResponse {
    status: CanisterStatus,
    memory_size: Nat,
    cycles: Nat,
    settings: CanisterSettings,
    module_hash: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Debug)]
enum CanisterStatus {
    running,
    stopping,
    stopped,
}

#[derive(CandidType, Deserialize, Debug)]
struct CanisterSettings {
    controllers: Vec<Principal>,
    compute_allocation: Nat,
    memory_allocation: Nat,
    freezing_threshold: Nat,
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
struct CanisterStatusRequest {
    canister_id: Principal,
}

#[derive(CandidType, Deserialize, Debug)]
struct Empty {}

#[derive(CandidType, Deserialize, Debug)]
struct Principals {
    principals: Vec<Principal>,
}

#[derive(CandidType, Deserialize)]
struct CanisterInfo {
    root_canister_id: Option<Principal>,
    governance_canister_id: Option<Principal>,
    index_canister_id: Option<Principal>,
    swap_canister_id: Option<Principal>,
    ledger_canister_id: Option<Principal>,
}

#[derive(CandidType, Deserialize)]
struct DeployedSnses {
    instances: Vec<CanisterInfo>,
}

#[derive(CandidType, Deserialize, Debug)]
struct SnsCanisters {
    root: Option<Principal>,
    swap: Option<Principal>,
    ledger: Option<Principal>,
    index: Option<Principal>,
    governance: Option<Principal>,
    dapps: Vec<Principal>,
    archives: Vec<Principal>,
}

#[derive(CandidType, Deserialize, Debug)]
struct DetailedCanisterStatusResponse {
    status: CanisterStatus,
    memory_size: Nat,
    cycles: Nat,
    settings: DetailedCanisterSettings,
    module_hash: Option<Vec<u8>>,
    reserved_cycles_limit: Option<Nat>,
    idle_cycles_burned_per_day: Option<Nat>,
    reserved_cycles: Option<Nat>,
}

#[derive(CandidType, Deserialize, Debug)]
struct DetailedCanisterSettings {
    controllers: Vec<Principal>,
    compute_allocation: Option<Nat>,
    memory_allocation: Option<Nat>,
    freezing_threshold: Option<Nat>,
}

#[derive(CandidType, Deserialize, Debug)]
struct SnsMetadata {
    url: Option<String>,
    logo: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

/// Fetches the root canisters from the NNS canister.
///
/// This function calls the `list_deployed_snses` method on the NNS canister to
/// retrieve the list of deployed SNS canisters, and then extracts the root
/// canister IDs from the response.
///
/// # Returns
/// A `Result` containing a `Vec` of `Principal` values representing the root
/// canister IDs, or an error message as a `String` if the call to
/// `list_deployed_snses` fails.
#[update]
async fn fetch_root_canisters() -> Result<Vec<Principal>, String> {
    let nns_canister = Principal::from_text("qaa6y-5yaaa-aaaaa-aaafa-cai").unwrap();

    // Call list_deployed_snses to get root canisters
    let result: Result<(DeployedSnses,), _> =
        call(nns_canister, "list_deployed_snses", (Empty {},)).await;

    match result {
        Ok((deployed_snses,)) => Ok(deployed_snses
            .instances
            .iter()
            .map(|x| x.root_canister_id.unwrap())
            .collect()),
        Err(e) => Err(format!("Error calling list_deployed_snses: {:?}", e)),
    }
}

/// Fetches the list of SNS canisters associated with the given root canister ID.
///
/// This function calls the `list_sns_canisters` method on the root canister to
/// retrieve the list of SNS canisters that are part of the SNS system rooted at
/// the given canister ID.
///
/// # Arguments
/// * `root_canister_id` - The Principal ID of the root canister for the SNS system.
///
/// # Returns
/// A `Result` containing the `SnsCanisters` struct, which holds the IDs of the
/// various SNS canisters (root, swap, ledger, index, governance, dapps, and
/// archives), or an error message as a `String` if the call to `list_sns_canisters`
/// fails.
#[update]
async fn fetch_sns_canisters_for_root(root_canister_id: Principal) -> Result<SnsCanisters, String> {
    let sns_canisters: Result<(SnsCanisters,), _> =
        call(root_canister_id.clone(), "list_sns_canisters", (Empty {},)).await;

    match sns_canisters {
        Ok((sns_canisters,)) => Ok(sns_canisters),
        Err(e) => {
            ic_cdk::print(format!("Error fetching sns canisters: {:?}", e));
            Err(format!("Error fetching sns canisters: {:?}", e))
        }
    }
}

/// Fetches the total cycles burn rate for all SNS canisters associated with the given root canister ID.
///
/// This function calls the `fetch_sns_canisters_for_root` function to get the list of SNS canisters
/// associated with the given root canister ID, and then iterates over those canisters to fetch the
/// cycles balance for each one using the `get_canister_cycles_from_root` function. The total cycles
/// burn rate across all SNS canisters is then returned.
///
/// # Arguments
/// * `root_canister_id` - The Principal ID of the root canister for the SNS system.
///
/// # Returns
/// A `Result` containing the total cycles burn rate across all SNS canisters associated with the
/// given root canister ID, or an error message as a `String` if any of the individual canister
/// cycle fetch operations fail.
#[update]
async fn get_root_canister_cycles_burn_rate(root_canister_id: Principal) -> Result<u64, String> {
    // Call fetch_sns_canisters_for_root to get the SNS canisters associated with the root canister
    let sns_canisters_result = fetch_sns_canisters_for_root(root_canister_id).await;

    match sns_canisters_result {
        Ok(sns_canisters) => {
            // Iterate over the SNS canisters and get the cycles balance for each
            let mut total_cycle_burn_rate: u64 = 0;

            let canister_ids = vec![
                sns_canisters.root,
                sns_canisters.swap,
                sns_canisters.ledger,
                sns_canisters.index,
                sns_canisters.governance,
            ]
            .into_iter()
            .flatten()
            .chain(sns_canisters.dapps.into_iter());
            // .chain(sns_canisters.archives.into_iter());

            for canister_id in canister_ids {
                match get_canister_cycles_from_root(root_canister_id, canister_id).await {
                    Ok(cycles) => total_cycle_burn_rate += cycles,
                    Err(e) => {
                        ic_cdk::print(format!(
                            "Error fetching cycles for canister {}: {}",
                            canister_id, e
                        ));
                    }
                }
            }

            Ok(total_cycle_burn_rate)
        }
        Err(e) => {
            ic_cdk::print(format!("Error fetching SNS canisters: {}", e));
            Err(format!("Error fetching SNS canisters: {}", e))
        }
    }
}

// get sns metadata from the governance canister
#[update]
async fn get_sns_metadata(root_canister_id: Principal) -> Result<SnsMetadata, String> {
    let sns_canisters_result = fetch_sns_canisters_for_root(root_canister_id).await;

    match sns_canisters_result {
        Ok(sns_canisters) => {
            let governance_canister = sns_canisters.governance;

            match governance_canister {
                Some(governance_canister) => {
                    let governance_result =
                        call(governance_canister, "get_metadata", (Empty {},)).await;
                    match governance_result {
                        Ok((sns_metadata,)) => Ok(sns_metadata),
                        Err(e) => {
                            ic_cdk::print(format!("Error fetching SNS metadata: {:?}", e));
                            return Err(format!("Error fetching SNS metadata: {:?}", e));
                        }
                    }
                }
                None => {
                    ic_cdk::print("No governance canister found");
                    return Err("No governance canister found".to_string());
                }
            }
        }
        Err(e) => {
            ic_cdk::print(format!("Error fetching SNS canisters: {}", e));
            Err(format!("Error fetching SNS canisters: {}", e))
        }
    }
}

/// Fetches the cycles burn rate for a given root canister ID by calling the `canister_status` method on each associated canister.
///
/// # Arguments
/// * `root_canister_id` - The Principal ID of the root canister for the SNS system.
///
/// # Returns
/// A `Result` containing the total cycles burn rate across all SNS canisters associated with the
/// given root canister ID, or an error message as a `String` if any of the individual canister
/// cycle fetch operations fail.
#[update]
async fn get_canister_cycles_from_root(
    root_canister_id: Principal,
    canister_id: Principal,
) -> Result<u64, String> {
    let request = CanisterStatusRequest {
        canister_id: canister_id,
    };

    let status_result: CallResult<(DetailedCanisterStatusResponse,)> =
        call(root_canister_id, "canister_status", (request,)).await;

    ic_cdk::print(format!("Raw response: {:?}", status_result));

    match status_result {
        Ok((canister_status,)) => {
            ic_cdk::print(format!("Decoded response: {:?}", canister_status));
            match canister_status.idle_cycles_burned_per_day {
                Some(idle_cycles_burned_per_day) => {
                    Ok(idle_cycles_burned_per_day.0.to_u64_digits().iter().sum())
                }
                None => {
                    ic_cdk::print(format!("Canister {} has no cycles balance", canister_id));
                    Err(format!("Canister {} has no cycles balance", canister_id))
                }
            }
        }
        Err((rejection_code, message)) => {
            ic_cdk::print(format!("Error fetching canister status: {:?}", message));
            Err(format!("Error fetching canister status: {:?}", message))
        }
    }
}

// #[update]
// async fn fetch_all_sns_canisters() -> Vec<Principal> {
//     let root_canisters_result = fetch_root_canisters().await;
//     let mut all_sns_canisters = Vec::new();

//     match root_canisters_result {
//         Ok(root_canisters) => {
//             for root_canister_id in root_canisters {
//                 match fetch_sns_canisters_for_root(root_canister_id).await {
//                     Ok(sns_canisters) => {
//                         if let Some(root) = sns_canisters.root {
//                             all_sns_canisters.push(root);
//                         }
//                         if let Some(swap) = sns_canisters.swap {
//                             all_sns_canisters.push(swap);
//                         }
//                         if let Some(ledger) = sns_canisters.ledger {
//                             all_sns_canisters.push(ledger);
//                         }
//                         if let Some(index) = sns_canisters.index {
//                             all_sns_canisters.push(index);
//                         }
//                         if let Some(governance) = sns_canisters.governance {
//                             all_sns_canisters.push(governance);
//                         }
//                         all_sns_canisters.extend(sns_canisters.dapps);
//                         all_sns_canisters.extend(sns_canisters.archives);
//                     }
//                     Err(e) => {
//                         ic_cdk::print(e);
//                     }
//                 }
//             }
//         }
//         Err(e) => {
//             ic_cdk::print(format!("Error fetching root canisters: {}", e));
//         }
//     }

//     all_sns_canisters
// }

/// Fetches the status of the specified canister, including the number of cycles it has.
///
/// # Arguments
/// * `canister_id` - The Principal ID of the canister to fetch the status for.
///
/// # Returns
/// A `CallResult` containing a vector of the canister's cycle count. If an error occurs, the `CallResult` will contain the rejection code and an error message.
#[update]
async fn get_canister_status(canister_id: Principal) -> CallResult<Vec<u64>> {
    let black_hole_canister_id = Principal::from_text("e3mmv-5qaaa-aaaah-aadma-cai").unwrap();

    let request = CanisterStatusRequest {
        canister_id: canister_id,
    };

    let status_result: CallResult<(CanisterStatusResponse,)> =
        call(black_hole_canister_id, "canister_status", (request,)).await;

    ic_cdk::print(format!("Raw response: {:?}", status_result));

    match status_result {
        Ok((canister_status,)) => {
            ic_cdk::print(format!("Decoded response: {:?}", canister_status));
            Ok(canister_status.cycles.0.to_u64_digits())
        }
        Err((rejection_code, message)) => {
            if message.contains("InvalidResponse") {
                Err((
                    rejection_code,
                    "Received an invalid response from the blackhole canister".to_string(),
                ))
            } else {
                Err((rejection_code, message))
            }
        }
    }
}

/// Updates the average burn rate of cycles based on the new cycles value.
///
/// This function calculates the difference between the new cycles value and the previous cycles value,
/// and updates the average burn rate accordingly. If the previous cycles value is 0 or the new cycles
/// value is greater than or equal to the previous cycles value, the average burn rate is kept constant.
///
/// # Arguments
/// * `new_cycles` - The new cycles value to update the average burn rate with.
///
/// # Returns
/// This function does not return a value, it updates the average burn rate in-place.
#[update]
fn update_burn_rate(new_cycles: u64) -> () {
    PREV_CYCLES.with(|prev_cycles| {
        AVG_BURN_RATE.with(|avg_burn_rate| {
            // store the difference between the new cycles an the old cycles in diff then find the average
            DIFF.with(|diff| {
                if *prev_cycles.borrow() == 0 || new_cycles >= *prev_cycles.borrow() {
                    // If prev_cycles is 0 or new_cycles is greater than or equal to prev_cycles,
                    // keep the avg_burn_rate constant
                    *prev_cycles.borrow_mut() = new_cycles;
                    return *avg_burn_rate.borrow();
                }

                let mut diff = diff.borrow_mut();
                diff.push((*prev_cycles.borrow()) as f64 - (new_cycles as f64));
                let sum: f64 = diff.iter().sum();
                *avg_burn_rate.borrow_mut() = sum / diff.len() as f64;
                *prev_cycles.borrow_mut() = new_cycles;
                *avg_burn_rate.borrow()
            });
        })
    })
}

/// Calculates the canister emission rate based on the network burn rate, network emission rate, and SNS burn rate.
///
/// If the network burn rate is 0.0, this function will return 0.0.
///
/// # Arguments
/// * `network_burn_rate` - The current network burn rate.
/// * `network_emission_rate` - The current network emission rate.
/// * `sns_burn_rate` - The current SNS burn rate.
///
/// # Returns
/// The calculated canister emission rate.
#[update]
fn calculate_canister_emission_rate(
    network_burn_rate: f64,
    network_emission_rate: f64,
    sns_burn_rate: f64,
) -> f64 {
    if network_burn_rate == 0.0 {
        return 0.0;
    }
    (sns_burn_rate * network_emission_rate) / network_burn_rate
}

ic_cdk::export_candid!();
