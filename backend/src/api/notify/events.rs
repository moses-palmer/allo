use crate::prelude::*;

use serde::{Deserialize, Serialize};

use crate::db::entities::user;
use crate::db::entities::{Allowance, Request, User};
use crate::db::values::UID;

/// An event sent over the notification channel.
#[derive(Clone, Debug, Message, Deserialize, Serialize)]
#[rtype(result = "()")]
#[serde(tag = "type")]
pub enum Event {
    /// An empty ping message.
    Ping {},

    /// The user has logged out.
    Logout {},

    /// The allowance for a user was updated.
    AllowanceUpdated {
        /// The new allowance.
        allowance: Allowance,

        /// The unique ID of the parent that updated the allowance.
        by: UID,
    },

    /// A family member was added.
    FamilyMemberInvited {
        /// The user that was added.
        user: user::UserDescription,

        /// The unique ID of the parent that added the family member.
        by: UID,
    },

    /// A family member was added.
    FamilyMemberAdded {
        /// The user that was added.
        user: User,

        /// The unique ID of the parent that added the family member.
        by: UID,
    },

    /// A family member was removed.
    FamilyMemberRemoved {
        /// The user that was removed.
        user: User,

        /// The unique ID of the parent that removed the family member.
        by: UID,
    },

    /// A request was made.
    RequestCreated {
        /// The request that was made.
        request: Request,

        /// The unique ID of the child that made the request.
        by: UID,
    },

    /// A request was granted.
    RequestGranted {
        /// The request that was granted.
        request: Request,

        /// The unique ID of the parent that granted the request.
        by: UID,
    },

    /// A request was declined.
    RequestDeclined {
        /// The request that was declined.
        request: Request,

        /// The unique ID of the parent that declined the request.
        by: UID,
    },
}
