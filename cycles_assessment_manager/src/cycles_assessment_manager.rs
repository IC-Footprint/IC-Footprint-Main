use candid::{CandidType, Decode, Deserialize, Nat, Principal};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::{query, update};

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

#[update]
async fn get_canister_status(canister_id: Principal) -> CallResult<(CanisterStatusResponse,)> {
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
            Ok((canister_status,))
        }
        Err((rejection_code, message)) => Err((rejection_code, message)),
    }
}

ic_cdk::export_candid!();
