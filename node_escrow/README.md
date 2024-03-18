# Node Escrow Module
The Node Escrow module provides functionality for managing payments and interactions with a ledger canister.

## Initialization
* init(conf: Conf): Initializes the escrow with configuration details such as the ledger canister ID, node ID, and ticket price.
    - Queries
        -  **get_ticket_price()**: Returns the current ticket price.
        - **get_price(ticket_count: u64)**: Calculates and returns the total price for a given number of tickets
        - **get_purchases()**: Returns a list of all purchases made.
    - Updates
        - **register_payment(ticket_count: u64)**: Registers a payment for a specified number of tickets. This involves transferring funds from the caller to the escrow canister and recording the payment.
    - Pre-Upgrade and Post-Upgrade Hooks
        - **pre_upgrade()**: Saves the current state of the escrow (payments, ledger canister ID, ticket price, current payment ID, and node ID) before an upgrade.
        - **post_upgrade()**: Restores the saved state after an upgrade.

# Cawa Poster Module
The Cawa Poster module facilitates interactions with the Cawa API for managing contributions and entities.

**NB**: Before interacting with the Cawa API, we have to create an entity for the node.

  #### PUT Request to Create an Entity

Endpoint:
```bash
PUT https://api.cawa.tech/api/v1/entity
```

Headers:
```bash
Content-Type: application/json
Authorization: Bearer <token>
```

Request Body:

```bash
{
 "name": "string", # node_escrow canister id
 "email": "string" # cawa+{canister_id}@carboncrowd.io
}
```
Response:

```bash
{
  "success": true,
  "errors": [
    "string"
  ],
  "id": "497f6eca-6276-4993-bfeb-53cbbbba6f08"
}
```

The rest of the endpoints are done on the canister.

* Updates
    - **set_api_key(api_key: String)**: Sets the API key for making requests to the Cawa API.
    - **authorize(principal: Principal)**: Authorizes a principal to set the API key.
    - **deauthorize(principal: Principal)**: Deauthorizes a principal from setting the API key.
    - **send(node_id: String, ticket_count: u64)**: Sends a contribution request to the Cawa API for a specified number of tickets.
    - **get_contributions_by_entity(node_id: String)**: Retrieves contributions made on behalf of a specific node
* Queries
    - **transform(raw: TransformArgs)**: Transforms the raw HTTP response from the Cawa API into a more readable format.