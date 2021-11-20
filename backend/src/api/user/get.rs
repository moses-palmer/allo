use sqlx::prelude::*;

use actix_session::Session;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Allowance, Entity, User};
use crate::db::values::UID;

/// Retrieves information about a user.
#[get("user/{user_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
    path: web::Path<UID>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let user_uid = path.into_inner();
    {
        let res = execute(&mut trans, state, &user_uid).await?;
        trans.commit().await?;
        Ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    state: State,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = api::expect(User::read(&mut *e, user_uid).await?)?;
    state.assert_family(&user.family_uid())?;
    let allowance = Allowance::read_for_user(&mut *e, user.uid())
        .await?
        .into_iter()
        .next();

    Ok(Res { user, allowance })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The user.
    user: User,

    /// The allowance schedule.
    allowance: Option<Allowance>,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_pool;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, _, _) = tests::populate(&mut c).unwrap();
        let allowance = create::allowance(
            &mut c,
            children.0.uid(),
            43,
            "mon".parse().unwrap(),
        );

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            &children.0.uid().clone(),
        )
        .await
        .unwrap();

        assert_eq!(res.user, children.0);
        assert_eq!(res.allowance, Some(allowance));
    }

    #[actix_rt::test]
    async fn success_parent_self() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            &parent.uid().clone(),
        )
        .await
        .unwrap();

        assert_eq!(res.user, parent);
        assert_eq!(res.allowance, None);
    }

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();
        let allowance = create::allowance(
            &mut c,
            children.0.uid(),
            43,
            "mon".parse().unwrap(),
        );

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            &children.0.uid().clone(),
        )
        .await
        .unwrap();

        assert_eq!(res.user, children.0);
        assert_eq!(res.allowance, Some(allowance));
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

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &other_child.uid().clone(),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }
}
