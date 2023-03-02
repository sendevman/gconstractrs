use crate::state::{Limits, Object, Pagination};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, StdResult};
use cosmwasm_std::{StdError, Uint128};

/// ObjectId is the type of identifier of an object in the bucket.
pub type ObjectId = String;

/// Cursor is the opaque type of cursor used for pagination.
pub type Cursor = String;

/// Instantiate messages
#[cw_serde]
pub struct InstantiateMsg {
    /// The name of the bucket.
    /// The name could not be empty or contains whitespaces.
    /// If name contains whitespace, they will be removed.
    pub bucket: String,
    /// The limits of the bucket.
    pub limits: BucketLimits,
    /// The configuration for paginated query.
    pub pagination: PaginationConfig,
}

/// Execute messages
#[cw_serde]
pub enum ExecuteMsg {
    /// # StoreObject
    /// StoreObject store an object to the bucket and make the sender the owner of the object.
    /// The object is referenced by the hash of its content and this value is returned.
    /// If the object is already stored, an error is returned.
    /// If pin is true, the object is pinned for the sender.
    StoreObject { data: Binary, pin: bool },

    /// # ForgetObject
    /// ForgetObject first unpin the object from the bucket for the considered sender, then remove
    /// it from the storage if it is not pinned anymore.
    /// If the object is pinned for other senders, it is not removed from the storage and an error is returned.
    /// If the object is not pinned for the sender, this is a no-op.
    ForgetObject { id: ObjectId },

    /// # PinObject
    /// PinObject pins the object in the bucket for the considered sender. If the object is already pinned
    /// for the sender, this is a no-op.
    /// While an object is pinned, it cannot be removed from the storage.
    PinObject { id: ObjectId },

    /// # UnpinObject
    /// UnpinObject unpins the object in the bucket for the considered sender. If the object is not pinned
    /// for the sender, this is a no-op.
    /// The object can be removed from the storage if it is not pinned anymore.
    UnpinObject { id: ObjectId },
}

/// Query messages
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// # Bucket
    /// Bucket returns the bucket information.
    #[returns(BucketResponse)]
    Bucket {},

    /// # Object
    /// Object returns the object information with the given id.
    #[returns(ObjectResponse)]
    Object {
        /// The id of the object to get.
        id: ObjectId,
    },

    /// # Objects
    /// Objects returns the list of objects in the bucket with support for pagination.
    #[returns(ObjectsResponse)]
    Objects {
        /// The owner of the objects to get.
        address: Option<String>,
        /// The number of objects to return.
        first: Option<u32>,
        /// The point in the sequence to start returning objects.
        after: Option<Cursor>,
    },

    /// # ObjectData
    /// ObjectData returns the content of the object with the given id.
    #[returns(Binary)]
    ObjectData {
        /// The id of the object to get.
        id: ObjectId,
    },

    /// # ObjectPins
    /// ObjectPins returns the list of addresses that pinned the object with the given id with
    /// support for pagination.
    #[returns(ObjectPinsResponse)]
    ObjectPins {
        /// The id of the object to get the pins for.
        id: ObjectId,
        /// The number of pins to return.
        first: Option<u32>,
        /// The point in the sequence to start returning pins.
        after: Option<Cursor>,
    },
}

/// # PageInfo
/// PageInfo is the page information returned for paginated queries.
#[cw_serde]
pub struct PageInfo {
    /// Tells if there is a next page.
    pub has_next_page: bool,
    /// The cursor to the next page.
    pub cursor: Cursor,
}

/// # BucketResponse
/// BucketResponse is the response of the Bucket query.
#[cw_serde]
pub struct BucketResponse {
    /// The name of the bucket.
    pub name: String,
    /// The limits of the bucket.
    pub limits: BucketLimits,
    /// The configuration for paginated query.
    pub pagination: PaginationConfig,
}

/// BucketLimits is the type of the limits of a bucket.
///
/// The limits are optional and if not set, there is no limit.
#[cw_serde]
pub struct BucketLimits {
    /// The maximum total size of the objects in the bucket.
    pub max_total_size: Option<Uint128>,
    /// The maximum number of objects in the bucket.
    pub max_objects: Option<Uint128>,
    /// The maximum size of the objects in the bucket.
    pub max_object_size: Option<Uint128>,
    /// The maximum number of pins in the bucket for an object.
    pub max_object_pins: Option<Uint128>,
}

impl BucketLimits {
    pub const fn new() -> Self {
        BucketLimits {
            max_total_size: None,
            max_objects: None,
            max_object_size: None,
            max_object_pins: None,
        }
    }

    pub fn set_max_total_size(mut self, max_total_size: Uint128) -> Self {
        self.max_total_size = Some(max_total_size);
        self
    }

    pub fn set_max_objects(mut self, max_objects: Uint128) -> Self {
        self.max_objects = Some(max_objects);
        self
    }

    pub fn set_max_object_size(mut self, max_object_size: Uint128) -> Self {
        self.max_object_size = Some(max_object_size);
        self
    }

    pub fn set_max_object_pins(mut self, max_object_pins: Uint128) -> Self {
        self.max_object_pins = Some(max_object_pins);
        self
    }
}

const MAX_PAGE_MAX_SIZE: u32 = u32::MAX - 1;
const DEFAULT_PAGE_MAX_SIZE: u32 = 30;
const DEFAULT_PAGE_DEFAULT_SIZE: u32 = 10;

/// PaginationConfig is the type carrying configuration for paginated queries.
///
/// The fields are optional and if not set, there is a default configuration.
#[cw_serde]
pub struct PaginationConfig {
    /// The maximum elements a page can contains.
    ///
    /// Shall be less than `u32::MAX - 1`.
    /// Default to '30' if not set.
    pub max_page_size: Option<u32>,
    /// The default number of elements in a page.
    ///
    /// Shall be less or equal than `max_page_size`.
    /// Default to '10' if not set.
    pub default_page_size: Option<u32>,
}

impl PaginationConfig {
    pub const fn new() -> Self {
        PaginationConfig {
            max_page_size: None,
            default_page_size: None,
        }
    }

    pub fn validate(&self) -> StdResult<()> {
        if self
            .max_page_size
            .filter(|size| size > &MAX_PAGE_MAX_SIZE)
            .is_some()
        {
            return Err(StdError::generic_err(
                "'max_page_size' cannot exceed 'u32::MAX - 1'",
            ));
        }

        if self
            .default_page_size
            .filter(|size| size > &self.max_page_size.unwrap_or(DEFAULT_PAGE_MAX_SIZE))
            .is_some()
        {
            return Err(StdError::generic_err(
                "'default_page_size' cannot exceed 'max_page_size'",
            ));
        }

        Ok(())
    }

    pub fn set_max_page_size(mut self, max_page_size: u32) -> Self {
        self.max_page_size = Some(max_page_size);
        self
    }

    pub fn set_default_page_size(mut self, default_page_size: u32) -> Self {
        self.default_page_size = Some(default_page_size);
        self
    }
}

impl From<Pagination> for PaginationConfig {
    fn from(value: Pagination) -> Self {
        PaginationConfig {
            max_page_size: Some(value.max_page_size),
            default_page_size: Some(value.default_page_size),
        }
    }
}

impl From<PaginationConfig> for Pagination {
    fn from(value: PaginationConfig) -> Self {
        Pagination {
            max_page_size: value.max_page_size.unwrap_or(DEFAULT_PAGE_MAX_SIZE),
            default_page_size: value.default_page_size.unwrap_or(DEFAULT_PAGE_DEFAULT_SIZE),
        }
    }
}

/// # ObjectResponse
/// ObjectResponse is the response of the Object query.
#[cw_serde]
pub struct ObjectResponse {
    /// The id of the object.
    pub id: ObjectId,
    /// The owner of the object.
    pub owner: String,
    /// Tells if the object is pinned by at least one address.
    pub is_pinned: bool,
    /// The size of the object.
    pub size: Uint128,
}

impl From<&Object> for ObjectResponse {
    fn from(object: &Object) -> Self {
        ObjectResponse {
            id: object.id.clone(),
            size: object.size,
            owner: object.owner.clone().into(),
            is_pinned: object.pin_count > Uint128::zero(),
        }
    }
}

/// # ObjectsResponse
/// ObjectsResponse is the response of the Objects query.
#[cw_serde]
pub struct ObjectsResponse {
    /// The list of objects in the bucket.
    pub data: Vec<ObjectResponse>,
    /// The page information.
    pub page_info: PageInfo,
}

/// # ObjectPinsResponse
/// ObjectPinsResponse is the response of the GetObjectPins query.
#[cw_serde]
pub struct ObjectPinsResponse {
    /// The list of addresses that pinned the object.
    pub data: Vec<String>,
    /// The page information.
    pub page_info: PageInfo,
}
