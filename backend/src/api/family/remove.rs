use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::{delete, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, User};
use crate::db::values::{Role, UID};
use crate::notifications::Notifier;

/// Removes a member from a family.
#[delete("family/{family_uid}/{user_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
    path: web::Path<(UID, UID)>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let (family_uid, user_uid) = path.into_inner();
    {
        let res =
            execute(&mut trans, state.clone(), &family_uid, &user_uid).await?;
        Notify::Family {
            event: Event::FamilyMemberRemoved {
                user: res.user.clone(),
                by: state.user_uid.clone(),
            },
            family: state.family_uid,
        }
        .send(&mut *trans, &notifier, &state.user_uid)
        .await;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    state: State,
    family_uid: &UID,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = User::read(&mut *e, user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown user"))?;
    let state = state
        .assert_role(Role::Parent)?
        .assert_family(family_uid)?
        .assert_family(user.family_uid())?;
    if &state.user_uid == user.uid() {
        Err(api::Error::forbidden("cannot remove self"))
    } else {
        user.delete(&mut *e).await?;
        Ok(Res { user })
    }
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The family member that was removed.
    user: User,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            children.0.family_uid(),
            children.0.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert!(User::read(&mut c, children.0.uid())
            .await
            .unwrap()
            .is_none());
    }

    #[actix_rt::test]
    async fn forbidden_parent_self() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            parent.family_uid(),
            parent.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("cannot remove self"));
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();
        let other_family = create::family(&mut c, "Other Family");
        let other_child = create::user(
            &mut c,
            Role::Child,
            "Other User",
            "other@email.com",
            other_family.uid(),
        );

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            other_child.family_uid(),
            other_child.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: children.0.uid().clone(),
                family_uid: family.uid().clone(),
                role: children.0.role().clone(),
            },
            children.1.family_uid(),
            children.1.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
