use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::{put, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db;
use crate::db::entities::{allowance, Allowance, Entity, User};
use crate::db::values::{Role, UID};
use crate::notifications::Notifier;

/// Changes the allowance for a user.
#[put("user/{user_uid}/allowance/{allowance_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
    req: web::Json<Req>,
    path: web::Path<(UID, UID)>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let (user_uid, allowance_uid) = path.into_inner();
    {
        let res = execute(
            &mut trans,
            state.clone(),
            &req.into_inner(),
            &user_uid,
            &allowance_uid,
        )
        .await?;
        Notify::MemberAndParents {
            event: Event::AllowanceUpdated {
                allowance: res.allowance.clone(),
                by: state.user_uid.clone(),
            },
            uid: user_uid,
            family: state.family_uid,
        }
        .send(&mut *trans, &notifier, &state.user_uid)
        .await;
        trans.commit().await?;
        Ok(res)
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    state: State,
    req: &Req,
    user_uid: &UID,
    allowance_uid: &UID,
) -> Result<Res, api::Error> {
    let user = api::expect(User::read(&mut *trans, user_uid).await?)?;
    state
        .assert_family(&user.family_uid())?
        .assert_role(Role::Parent)?;

    let allowance =
        api::expect(Allowance::read(&mut *trans, &allowance_uid).await?)?
            .merge(req.clone().merge(allowance::Description {
                user_uid: Some(user.uid().clone()),
                ..Default::default()
            }));
    allowance.update(&mut *trans).await?;

    Ok(Res { allowance })
}

pub type Req = allowance::Description;

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The new allowance.
    allowance: Allowance,
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

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, _, _) = tests::populate(&mut c).unwrap();
        let allowance = create::allowance(
            &mut c,
            children.0.uid(),
            42,
            "mon".parse().unwrap(),
        );

        let mut trans = c.begin().await.unwrap();
        execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                amount: Some(84),
                schedule: Some("tue".parse().unwrap()),
                ..Default::default()
            },
            allowance.user_uid(),
            allowance.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        let allowance = Allowance::read(&mut c, allowance.uid())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(allowance.user_uid(), children.0.uid());
        assert_eq!(allowance.amount(), &84);
        assert_eq!(allowance.schedule(), &"tue".parse().unwrap());
    }

    #[actix_rt::test]
    async fn success_user_uid_ignored() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, _, _) = tests::populate(&mut c).unwrap();
        let allowance = create::allowance(
            &mut c,
            children.0.uid(),
            42,
            "mon".parse().unwrap(),
        );

        let mut trans = c.begin().await.unwrap();
        execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                user_uid: Some(children.1.uid().clone()),
                amount: Some(84),
                schedule: Some("tue".parse().unwrap()),
                ..Default::default()
            },
            allowance.user_uid(),
            allowance.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        let allowance = Allowance::read(&mut c, allowance.uid())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(allowance.user_uid(), children.0.uid());
        assert_eq!(allowance.amount(), &84);
        assert_eq!(allowance.schedule(), &"tue".parse().unwrap());
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();
        let other_family = create::family(&mut c, "Other Family");
        let other_child = create::user(
            &mut c,
            Role::Child,
            "Other User",
            "other@email.com",
            other_family.uid(),
        );
        let allowance = create::allowance(
            &mut c,
            other_child.uid(),
            42,
            "mon".parse().unwrap(),
        );

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: children.0.uid().clone(),
                family_uid: family.uid().clone(),
                role: children.0.role().clone(),
            },
            &Req {
                amount: Some(84),
                schedule: Some("tue".parse().unwrap()),
                ..Default::default()
            },
            allowance.user_uid(),
            allowance.uid(),
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
        let allowance = create::allowance(
            &mut c,
            children.0.uid(),
            42,
            "mon".parse().unwrap(),
        );

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: children.0.uid().clone(),
                family_uid: family.uid().clone(),
                role: children.0.role().clone(),
            },
            &Req {
                amount: Some(84),
                schedule: Some("tue".parse().unwrap()),
                ..Default::default()
            },
            allowance.user_uid(),
            allowance.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
