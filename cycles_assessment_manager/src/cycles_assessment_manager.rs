use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::update;
use std::cell::RefCell;

thread_local! {
    static CUMULATIVE_ENERGY: RefCell<f64> = RefCell::new(0.0);
    static CUMULATIVE_EMISSIONS: RefCell<f64> = RefCell::new(0.0);
    static PREV_CYCLES: RefCell<u64> = RefCell::new(0);
    static LAST_ENERGY_USAGE: RefCell<f64> = RefCell::new(0.0);
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
    Running,
    Stopping,
    Stopped,
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

const KJ_TO_KWH: f64 = 3_600_000.0;
const CARBON_EMISSION_PER_KWH: f64 = 0.5;

#[update]
async fn get_canister_status(canister_id: Principal) -> CallResult<Vec<u64>> {
    let black_hole_canister_id = Principal::from_text("bw4dl-smaaa-aaaaa-qaacq-cai").unwrap();

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
        Err((rejection_code, message)) => Err((rejection_code, message)),
    }
}

#[update(name = "record_energy_usage")]
pub fn record_energy_usage(current_cycles: Vec<u64>) -> f64 {
    PREV_CYCLES.with(|prev_cycles| {
        LAST_ENERGY_USAGE.with(|last_energy_usage| {
            let mut prev_cycles = prev_cycles.borrow_mut();
            let mut last_energy_usage = last_energy_usage.borrow_mut();
            let cycles = current_cycles[0];

            let energy_used = if *prev_cycles == 0 {
                0.0
            } else if cycles <= *prev_cycles {
                let cycles_diff = *prev_cycles - cycles;
                let energy = cycles_diff as f64 * 0.5;
                *last_energy_usage = energy;
                energy
            } else {
                *last_energy_usage
            };

            ic_cdk::println!("prev_cycles: {}, current_cycles: {}, energy_used: {}", *prev_cycles, cycles, energy_used);

            *prev_cycles = cycles;
            ic_cdk::println!("prev_cycles: {}", *prev_cycles);
            energy_used
        })
    })
}

#[update]
fn calculate_carbon_emissions(energy_used: f64) -> f64 {
    let energy_used_kwh = energy_used / KJ_TO_KWH;
    let carbon_emissions = energy_used_kwh * CARBON_EMISSION_PER_KWH;
    carbon_emissions
}

#[update]
fn update_cumulative_data(energy_used: f64, carbon_emissions: f64) {
    CUMULATIVE_ENERGY.with(|cumulative_energy| {
        *cumulative_energy.borrow_mut() += energy_used;
    });
    CUMULATIVE_EMISSIONS.with(|cumulative_emissions| {
        *cumulative_emissions.borrow_mut() += carbon_emissions;
    });
}

#[update]
pub async fn calculate_emissions(canister_id: Principal) -> Result<(f64, f64, f64), String> {
    match get_canister_status(canister_id).await {
        Ok(canister_status,) => {
            let current_cycles = canister_status;
            // let cycles = current_cycles[0];
            let energy_used = record_energy_usage(current_cycles);
            let carbon_emissions = calculate_carbon_emissions(energy_used);
            update_cumulative_data(energy_used, carbon_emissions);

            CUMULATIVE_ENERGY.with(|cumulative_energy| {
                CUMULATIVE_EMISSIONS.with(|cumulative_emissions| {
                    let cumulative_energy = *cumulative_energy.borrow();
                    let cumulative_emissions = *cumulative_emissions.borrow();
                    Ok((carbon_emissions, cumulative_energy, cumulative_emissions))
                })
            })
        }
        Err((rejection_code, message)) => {
            let error_message = format!("Failed to get canister status: {:?} - {}", rejection_code, message);
            Err(error_message)
        }
    }
}


ic_cdk::export_candid!();
