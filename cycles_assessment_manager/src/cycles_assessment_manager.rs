use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::{query, update};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static PREV_CYCLES: RefCell<u64> = RefCell::new(0);
    static AVG_BURN_RATE: RefCell<f64> = RefCell::new(0.0);
    static DIFF: RefCell<Vec<f64>> = RefCell::new(Vec::new());
    static SNS_DATA: RefCell<SnsData> = RefCell::new(SnsData::new());
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

#[derive(CandidType, Deserialize, Debug, Clone)]
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

#[derive(CandidType, Deserialize, Debug, Clone)]
struct SnsMetadata {
    url: Option<String>,
    logo: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(CandidType, Deserialize, Debug, Clone)]
struct SnsData {
    root_canisters: Vec<Principal>,
    canisters: HashMap<Principal, SnsCanisters>,
    cycle_burn_rate: HashMap<Principal, u64>,
    sns_emissions: HashMap<Principal, f64>,
    metadata: HashMap<Principal, SnsMetadata>,
    emissions_data: HashMap<Principal, SnsEmissionData>,
}

#[derive(CandidType, Deserialize, Debug, Clone)]
struct SnsEmissionData {
    last_calculation_time: u64,
    cumulative_emissions: f64,
}

impl SnsData {
    fn new() -> Self {
        SnsData {
            root_canisters: Vec::new(),
            canisters: HashMap::new(),
            cycle_burn_rate: HashMap::new(),
            sns_emissions: HashMap::new(),
            metadata: HashMap::new(),
            emissions_data: HashMap::new(),
        }
    }
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

    let result: Result<(DeployedSnses,), _> =
        call(nns_canister, "list_deployed_snses", (Empty {},)).await;

    match result {
        Ok((deployed_snses,)) => {
            let root_canisters: Vec<Principal> = deployed_snses
                .instances
                .iter()
                .map(|x| x.root_canister_id.unwrap())
                .collect();

            SNS_DATA.with(|data| {
                data.borrow_mut().root_canisters = root_canisters.clone();
            });

            Ok(root_canisters)
        }
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
        Ok((sns_canisters,)) => {
            SNS_DATA.with(|data| {
                data.borrow_mut()
                    .canisters
                    .insert(root_canister_id, sns_canisters.clone());
            });

            Ok(sns_canisters)
        }
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

#[update]
/// Fetches the SNS metadata from the governance canister for the given root canister ID.
///
/// # Arguments
/// * `root_canister_id` - The Principal ID of the root canister for the SNS system.
///
/// # Returns
/// A `Result` containing the SNS metadata, or an error message as a `String` if the metadata fetch operation fails.
async fn get_sns_metadata(root_canister_id: Principal) -> Result<SnsMetadata, String> {
    let sns_canisters_result = fetch_sns_canisters_for_root(root_canister_id).await;

    match sns_canisters_result {
        Ok(sns_canisters) => {
            if let Some(governance_canister) = sns_canisters.governance {
                let governance_result: CallResult<(SnsMetadata,)> =
                    call(governance_canister, "get_metadata", (Empty {},)).await;
                match governance_result {
                    Ok((sns_metadata,)) => {
                        SNS_DATA.with(|data| {
                            data.borrow_mut()
                                .metadata
                                .insert(root_canister_id, sns_metadata.clone());
                        });
                        Ok(sns_metadata)
                    }
                    Err(e) => {
                        ic_cdk::print(format!("Error fetching SNS metadata: {:?}", e));
                        Err(format!("Error fetching SNS metadata: {:?}", e))
                    }
                }
            } else {
                ic_cdk::print("No governance canister found");
                Err("No governance canister found".to_string())
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

    match status_result {
        Ok((canister_status,)) => match canister_status.idle_cycles_burned_per_day {
            Some(idle_cycles_burned_per_day) => {
                let burn_rate = idle_cycles_burned_per_day.0.to_u64_digits().iter().sum();
                SNS_DATA.with(|data| {
                    data.borrow_mut()
                        .cycle_burn_rate
                        .insert(canister_id, burn_rate);
                });
                Ok(burn_rate)
            }
            None => Err(format!("Canister {} has no cycles balance", canister_id)),
        },
        Err((_, message)) => Err(format!("Error fetching canister status: {:?}", message)),
    }
}

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
    let emission_rate = (sns_burn_rate * network_emission_rate) / network_burn_rate;
    emission_rate
}

#[update]
async fn get_cumulative_sns_emissions(
    root_id: Principal,
    network_burn_rate: f64,
    network_emission_rate: f64,
    sns_burn_rate: f64,
) -> Result<f64, String> {
    let current_time = ic_cdk::api::time();

    // First, read the current data without mutable borrow
    let (last_calculation_time, cumulative_emissions) = SNS_DATA.with(|data| {
        let data = data.borrow();
        let emissions_data =
            data.emissions_data
                .get(&root_id)
                .cloned()
                .unwrap_or(SnsEmissionData {
                    last_calculation_time: 0,
                    cumulative_emissions: 0.0,
                });
        (
            emissions_data.last_calculation_time,
            emissions_data.cumulative_emissions,
        )
    });

    let hours_passed = (current_time - last_calculation_time) / (60 * 60 * 1_000_000_000);

    if hours_passed >= 24 {
        // Calculate new daily emission
        let daily_emission = calculate_canister_emission_rate(
            network_burn_rate,
            network_emission_rate,
            sns_burn_rate,
        );

        let new_emissions = daily_emission;
        let new_cumulative_emissions = cumulative_emissions + new_emissions;

        // Update the data with a mutable borrow
        SNS_DATA.with(|data| {
            let mut data = data.borrow_mut();
            let emissions_data = data
                .emissions_data
                .entry(root_id)
                .or_insert(SnsEmissionData {
                    last_calculation_time: 0,
                    cumulative_emissions: 0.0,
                });
            emissions_data.cumulative_emissions = new_cumulative_emissions;
            emissions_data.last_calculation_time = current_time;
        });

        Ok(new_cumulative_emissions)
    } else {
        Ok(cumulative_emissions)
    }
}

#[query]
/// Returns a vector of all the root canister IDs in the SNS data.
fn get_root_canisters() -> Vec<Principal> {
    SNS_DATA.with(|data| data.borrow().root_canisters.clone())
}

#[query]
/// Returns the SNS canisters associated with the given root canister ID, if they exist.
///
/// # Arguments
/// * `root_canister_id` - The root canister ID to look up.
///
/// # Returns
/// An `Option` containing the `SnsCanisters` associated with the given root canister ID, if they exist.
fn get_sns_canisters(root_canister_id: Principal) -> Option<SnsCanisters> {
    SNS_DATA.with(|data| data.borrow().canisters.get(&root_canister_id).cloned())
}

#[query]
/// Returns the current cycle burn rate for the given canister ID, if it exists.
///
/// # Arguments
/// * `canister_id` - The ID of the canister to get the cycle burn rate for.
///
/// # Returns
/// An `Option` containing the cycle burn rate for the given canister ID, if it exists.
fn get_cycle_burn_rate(canister_id: Principal) -> Option<u64> {
    SNS_DATA.with(|data| data.borrow().cycle_burn_rate.get(&canister_id).cloned())
}

#[query]
/// Returns a clone of the entire SNS data.
///
/// This function retrieves the SNS data from the `SNS_DATA` thread-local storage and returns a clone of the entire data structure.
fn get_all_sns_data() -> SnsData {
    SNS_DATA.with(|data| data.borrow().clone())
}

#[query]
/// Returns the current SNS emissions for the given root canister ID, if they exist.
///
/// # Arguments
/// * `root_id` - The root canister ID to look up.
///
/// # Returns
/// An `Option` containing the current SNS emissions for the given root canister ID, if they exist.
fn get_sns_emissions(root_id: Principal) -> Option<f64> {
    SNS_DATA.with(|data| data.borrow().sns_emissions.get(&root_id).cloned())
}

#[query]
fn get_metadata(root_id: Principal) -> Option<SnsMetadata> {
    SNS_DATA.with(|data| data.borrow().metadata.get(&root_id).cloned())
}

ic_cdk::export_candid!();
