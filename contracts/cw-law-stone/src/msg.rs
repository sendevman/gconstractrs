use crate::state::Object;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
#[allow(unused_imports)]
use logic_bindings::AskResponse;

/// Instantiate message
#[cw_serde]
pub struct InstantiateMsg {
    /// The Prolog program carrying law rules and facts.
    pub program: Binary,

    /// The `cw-storage` contract address on which to store the law program.
    pub storage_address: String,
}

/// Execute messages
#[cw_serde]
pub enum ExecuteMsg {
    /// # BreakStone
    /// Break the stone making this contract unusable, by clearing all the related resources:
    /// - Unpin all the pinned objects on `cw-storage` contracts, if any.
    /// - Forget the main program (i.e. or at least unpin it).
    /// Only the contract admin is authorized to break it, if any.
    /// If already broken, this is a no-op.
    BreakStone,
}

/// Query messages
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// # Ask
    /// If not broken, ask the logic module the provided query with the law program loaded.
    #[returns(AskResponse)]
    Ask { query: String },

    /// # Program
    /// If not broken, returns the law program location information.
    #[returns(ProgramResponse)]
    Program,
}

/// # ProgramResponse
/// ProgramResponse carry elements to locate the program in a `cw-storage` contract.
#[cw_serde]
pub struct ProgramResponse {
    /// The program object id in the `cw-storage` contract.
    pub object_id: String,

    /// The `cw-storage` contract address on which the law program is stored.
    pub storage_address: String,
}

impl From<Object> for ProgramResponse {
    fn from(o: Object) -> Self {
        ProgramResponse {
            object_id: o.object_id,
            storage_address: o.storage_address,
        }
    }
}
