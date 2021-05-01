use actix_session::Session;
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::api;
use crate::db::values::{Role, UID};

pub mod introspect;
pub mod login;
pub mod logout;
pub mod password;

/// A log-in session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State {
    /// The currently logged in user.
    pub user_uid: UID,

    /// The family of the logged in user.
    pub family_uid: UID,

    /// The family role of the logged in user.
    pub role: Role,
}

impl State {
    /// The cookie containing the current session state.
    pub const COOKIE: &'static str = "state";

    /// Loads the state cookie from the session.
    ///
    /// If the cookie is not present, or cannot be parsed, 401 is returned.
    ///
    /// # Arguments
    /// *  `session` - The session.
    pub fn load(session: &Session) -> Result<Self, api::Error> {
        session
            .get::<Self>(Self::COOKIE)
            .ok()
            .flatten()
            .ok_or_else(api::Error::unauthorized)
    }

    /// Stores the state cookie in the session.
    ///
    /// # Arguments
    /// *  `session` - The session.
    pub fn store(&self, session: &Session) -> Result<(), api::Error> {
        session.set(Self::COOKIE, self).map_err(|_| {
            api::Error::Static(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to set state cookie",
            )
        })
    }

    /// Clears the state cookie from the session.
    ///
    /// # Arguments
    /// *  `session` - The session.
    pub fn clear(session: &Session) {
        session.purge();
    }

    /// Asserts that the currently logged in user is a specific user.
    ///
    /// # Arguments
    /// *  `user_uid` - The required user.
    pub fn assert_user(self, user_uid: &UID) -> Result<Self, api::Error> {
        if &self.user_uid == user_uid {
            Ok(self)
        } else {
            Err(api::Error::forbidden("invalid user"))
        }
    }

    /// Asserts that the currently logged in user is part of a specific family.
    ///
    /// # Arguments
    /// *  `family_uid` - The required family.
    pub fn assert_family(self, family_uid: &UID) -> Result<Self, api::Error> {
        if &self.family_uid == family_uid {
            Ok(self)
        } else {
            Err(api::Error::forbidden("invalid family"))
        }
    }

    /// Asserts that the currently logged in user has a specific role.
    ///
    /// # Arguments
    /// *  `role` - The required role.
    pub fn assert_role(self, role: Role) -> Result<Self, api::Error> {
        if self.role == role {
            Ok(self)
        } else {
            Err(api::Error::forbidden("invalid role"))
        }
    }
}
