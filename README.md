# Canister Setup


This setup is built around two main canisters: esg_wallet.rs and node_manager.rs.


## The Entry Point: 
### esg_wallet.rs

The esg_wallet.rs canister is the entry point and primary interface for the frontend. It manages payments and purchases. 

This canister is the wallet the user will interact with when trying to offset emissions. This is where the funds go to after a purchase is successful.

##### Methods:

**register_payment(ticket_count: u64, nodeId:** Option<String>):** 

Registers a payment. This method is public and can be called by anyone.

**get_ticket_price():** 

Returns the current ticket price. This method is public and can be called by anyone.


**get_price(ticket_count: f64):** 

Calculates and returns the price for a given ticket count. This method is public and can be called by anyone.

**get_purchases():** 

Returns all purchases. This method is public and can be called by anyone.

**set_offset_emissions(nodeId: Option<String>):** 

Sets offset emissions for a node. This method is public and can be called by any principal that is authorized.

**withdraw(wallet: Principal, amount: u64):** 

Withdraws payments. This method is public and can be called by any principal that is authorized.

### Contribution Handling: 
#### cawa_poster.rs


The cawa_poster.rs file is one of the several vendor set ups we will have. 

This file is dedicated to handling contributions to the Cawa platform. 

There are authorization mechanisms in place in the file to ensure that only specific principles can make these requests to cawa.

##### Methods

**set_api_key(api_key: String):** 

Sets the API key for Cawa. This method is public and can be called by any principal that is authorized.


**authorize(principal: Principal):**

 Authorizes a principal to perform certain actions. This method is public and can be called by any principal that is authorized.


**deauthorize(principal: Principal):**

Deauthorizes a principal, revoking their ability to perform certain actions. This method is public and can be called by any principal that is authorized.


**send(client: String, ticket_count: f64):** 

Sends a contribution to the Cawa platform. This method is public and can be called by any principal that is authorized.

**transform(raw: TransformArgs):** 

Transforms HTTP responses. This method is public and is automatically called by the IC when an HTTP request is made.

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


# Set Up

To deploy the esg Wallet canister make sure to pass the ledger canister id as an argument. ie

```bash
dfx deploy --argument '(record {ledger_canister_id = principal "ryjl3-tyaaa-aaaaa-aaaba-cai";})' esg_wallet --network ic
```
no arguments are necessary for the Node Management canister.

Once both canisters are deployed, change the .env values of the canister ids in the frontend repo, then run 

```bash
npm run build
``` 

Also make sure to change the canister id in the set_offset_emmissions method in the esg_wallet.rs to the new node_manager canister id.