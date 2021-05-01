use sqlx::prelude::*;

use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{allowance, user, Entity, Password, User};
use crate::db::values::{PasswordHash, Role, UID};

/// Adds a new member to a family.
///
/// This action is used to add both parents and children.
#[post("family/{family_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
    req: web::Json<Req>,
    path: web::Path<UID>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let family_uid = path.into_inner();
    {
        let res =
            execute(&mut trans, state, &req.into_inner(), &family_uid).await?;
        trans.commit().await?;
        Ok(res)
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    state: State,
    req: &Req,
    family_uid: &UID,
) -> Result<Res, api::Error> {
    state
        .assert_role(Role::Parent)?
        .assert_family(&family_uid)?;
    let user = api::argument(
        req.user
            .clone()
            .merge(user::Description {
                family_uid: Some(family_uid.clone()),
                ..Default::default()
            })
            .entity(UID::new()),
    )?;
    if user.role() == &Role::Parent && req.allowance.is_some() {
        return Err(api::Error::Static(
            StatusCode::BAD_REQUEST,
            "a parent cannot have an allowance",
        ));
    }
    if User::read_for_family(&mut *trans, &family_uid)
        .await?
        .iter()
        .any(|u| u.name() == user.name())
    {
        return Err(api::Error::Static(
            StatusCode::CONFLICT,
            "user already exists",
        ));
    }
    let password = Password::new(
        user.uid().clone(),
        api::argument(PasswordHash::from_password(&req.password))?,
    );

    user.create(&mut *trans).await?;
    password.create(&mut *trans).await?;
    if let Some(allowance) = req.allowance.clone() {
        api::argument(
            allowance
                .merge(allowance::Description {
                    user_uid: Some(user.uid().clone()),
                    ..Default::default()
                })
                .entity(UID::new()),
        )?
        .create(&mut *trans)
        .await?;
    }

    Ok(Res { user })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The user to add.
    pub user: user::Description,

    /// The user allowance if they are a child.
    pub allowance: Option<allowance::Description>,

    /// The user password.
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The generated user.
    pub user: User,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Created().json(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::entities::Allowance;
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                user: user::Description {
                    role: Some(Role::Child),
                    name: Some("child".into()),
                    email: Some(None),
                    ..Default::default()
                },
                allowance: Some(allowance::Description {
                    amount: Some(42),
                    schedule: Some("mon".parse().unwrap()),
                    ..Default::default()
                }),
                password: "123".into(),
            },
            family.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        let user = User::read(&mut c, res.user.uid()).await.unwrap().unwrap();
        assert_eq!(user, res.user);
        assert_eq!(user.role(), &Role::Child);
        assert_eq!(user.name(), "child");
        assert_eq!(user.email(), &None);

        let password =
            Password::read(&mut c, user.uid()).await.unwrap().unwrap();
        assert!(password.hash().verify("123").unwrap());

        let allowances =
            Allowance::read_for_user(&mut c, user.uid()).await.unwrap();
        assert_eq!(allowances.len(), 1);
        assert_eq!(allowances[0].amount(), &42);
        assert_eq!(allowances[0].schedule(), &"mon".parse().unwrap());
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();
        let other_family = create::family(&mut c, "Other Family");

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                user: user::Description {
                    role: Some(Role::Child),
                    name: Some("child".into()),
                    email: Some(None),
                    ..Default::default()
                },
                allowance: None,
                password: "123".into(),
            },
            other_family.uid(),
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
            &Req {
                user: user::Description {
                    role: Some(Role::Child),
                    name: Some("child".into()),
                    email: Some(None),
                    ..Default::default()
                },
                allowance: None,
                password: "123".into(),
            },
            family.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
