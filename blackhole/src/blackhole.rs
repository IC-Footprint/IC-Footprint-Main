use candid::{CandidType, Deserialize, Principal};
use ic_cdk::update;

#[derive(CandidType, Deserialize, Debug)]
struct CanisterSettings {
    controllers: Vec<Principal>,
    compute_allocation: candid::Nat,
    memory_allocation: candid::Nat,
    freezing_threshold: candid::Nat,
}

#[derive(CandidType, Deserialize, Debug)]
enum CanisterStatus {
    Running,
    Stopping,
    Stopped,
}

#[derive(CandidType, Deserialize, Debug)]
struct CanisterStatusResponse {
    status: CanisterStatus,
    memory_size: candid::Nat,
    cycles: candid::Nat,
    settings: CanisterSettings,
    module_hash: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Debug)]
struct CanisterStatusRequest {
    canister_id: Principal,
}

#[update]
async fn canister_status(request: CanisterStatusRequest) -> CanisterStatusResponse {
    let ic_canister_id = Principal::from_text("aaaaa-aa").unwrap();

    let status_result: Result<(CanisterStatusResponse,), _> =
        ic_cdk::call(ic_canister_id, "canister_status", (request,)).await;

    match status_result {
        Ok((canister_status,)) => canister_status,
        Err((_, err)) => ic_cdk::trap(&format!("Failed to get canister status: {}", err)),
    }
}

ic_cdk::export_candid!();