# ESG Wallet

Welcome to IC Footprint, a pioneering initiative leveraging blockchain technology to drive environmental sustainability. Our platform facilitates transparent and secure transactions, enabling individuals and organizations to offset emissions and contribute to a greener future

## Canister Setup

This setup is built around three main canisters: esg_wallet.rs, node_manager.rs, and cycles_assessment_manager.rs.

## The Entry Point:

### esg_wallet.rs

The esg_wallet.rs canister is the entry point and primary interface for the frontend. It manages payments and purchases.

This canister is the wallet the user will interact with when trying to offset emissions. This is where the funds go to after a purchase is successful.

##### Methods:

**register_payment(amount: u64):**

Registers a payment. This method is public and can be called by anyone.

**get_purchases():**

Returns all purchases. This method is public and can be called by anyone.

**set_offset_emissions(nodeId: Option<String>):**

Sets offset emissions for a node. This method is public and can be called by any principal that is authorized.

**withdraw(wallet: Principal, amount: u64):**

Withdraws payments. This method is public and can be called by any principal that is authorized.

### Node and Emission Management:

#### node_manager.rs

The node_manager.rs canister is responsible for managing nodes and their emissions.

##### Methods

**set_api_key(api_key: String):**

Sets the API key for node management. This method is public and can be called by any principal that is authorized.

**authorize(principal: Principal):**

Authorizes a principal to perform certain actions. This method is public and can be called by any principal that is authorized.

**deauthorize(principal: Principal):**

Deauthorizes a principal, revoking their ability to perform certain actions. This method is public and can be called by any principal that is authorized.

**get_emissions():**

Returns all nodes plus their emissions. This method is public and can be called by anyone.

**offset_emissions(client: Client, offset: f64, node_name: Option<String>):**

Offsets emissions from nodes based on a client. This method is public and can be called by any principal that is authorized.

**select_random_nodes():**

Selects a random set of nodes. This method is not public is only called within the canister itself.

**get_offset_emissions(simple_client: SimpleClient, payment: Vec<Payment>, node_name: Option<String>):**

Gets the offset emissions for a client and a list of payments. This method is public and can be called by any principal that is authorized.

**get_node_offset_emissions(node_name: String):**

Gets the offset emissions for a specific node. This method is public and can be called by anyone.

**get_client_offset_emissions(client_name: String):**

Gets the offset emissions for a specific client. This method is public and can be called by anyone.

**get_projects():**

Gets all projects. This method is public and can be called by anyone.

**add_project(project: Project):**

Adds a new project. This method is public and can be called by any principal that is authorized.

**remove_project(project_id: String):**

Removes a project by its ID. This method is public and can be called by any principal that is authorized.

**delete_all_projects():**

Deletes all projects. This method is public and can be called by any principal that is authorized.

### Cycles Assessment Management:

#### cycles_assessment_manager.rs

The cycles_assessment_manager.rs canister is responsible for managing the cycles assessment of the SNS canisters.

##### Methods

**fetch_root_canisters():**

Fetches the root canisters from the NNS canister. This method is public and can be called by any principal that is authorized.

**fetch_sns_canisters_for_root(root_canister_id: Principal):**

Fetches the list of SNS canisters associated with the given root canister ID. This method is public and can be called by any principal that is authorized.

**get_root_canister_cycles_burn_rate(root_canister_id: Principal):**

Fetches the total cycles burn rate for all SNS canisters associated with the given root canister ID. This method is public and can be called by any principal that is authorized.

**get_sns_metadata(root_canister_id: Principal):**

Fetches the SNS metadata from the governance canister for the given root canister ID. This method is public and can be called by any principal that is authorized.

**get_canister_cycles_from_root(root_canister_id: Principal, canister_id: Principal):**

Fetches the cycles burn rate for a given root canister ID by calling the `canister_status` method on each associated canister. This method is public and can be called by any principal that is authorized.

**get_canister_status(canister_id: Principal):**

Fetches the status of the specified canister, including the number of cycles it has. This method is public and can be called by any principal that is authorized.

**calculate_canister_emission_rate(network_burn_rate: f64, network_emission_rate: f64, sns_burn_rate: f64):**

Calculates the canister emission rate based on the network burn rate, network emission rate, and SNS burn rate. This method is public and can be called by any principal that is authorized.

**get_cumulative_sns_emissions(root_id: Principal, network_burn_rate: f64, network_emission_rate: f64, sns_burn_rate: f64):**

Gets the cumulative SNS emissions for a given root canister ID. This method is public and can be called by any principal that is authorized.

**get_root_canisters():**

Returns a vector of all the root canister IDs in the SNS data. This method is public and can be called by anyone.

**get_sns_canisters(root_canister_id: Principal):**

Returns the SNS canisters associated with the given root canister ID, if they exist. This method is public and can be called by anyone.

**get_cycle_burn_rate(canister_id: Principal):**

Returns the current cycle burn rate for the given canister ID, if it exists. This method is public and can be called by anyone.

**get_all_sns_data():**

Returns a clone of the entire SNS data. This method is public and can be called by anyone.

**get_sns_emissions(root_id: Principal):**

Returns the current SNS emissions for the given root canister ID, if they exist. This method is public and can be called by anyone.

**get_metadata(root_id: Principal):**

Returns the metadata for the given root canister ID, if it exists. This method is public and can be called by anyone.

**get_stored_sns_emissions(root_id: Principal):**

Returns the stored SNS emissions for the given root canister ID, if they exist. This method is public and can be called by anyone.

# Set Up

To deploy the esg Wallet canister make sure to pass the ledger canister id as an argument. ie

```
dfx deploy --argument '(record {ledger_canister_id = principal "ryjl3-tyaaa-aaaaa-aaaba-cai";})' esg_wallet --network ic
```

no arguments are necessary for the Node Management canister.

Once both canisters are deployed, change the .env values of the canister ids in the frontend repo, then run

```
npm run build
```

Also make sure to change the canister id in the set_offset_emmissions method in the esg_wallet.rs to the new node_manager canister id.
