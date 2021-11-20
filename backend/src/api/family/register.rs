use sqlx::prelude::*;

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::db;
use crate::db::entities::{family, user, Entity, Family, Password, User};
use crate::db::values::{PasswordHash, Role, UID};

/// Registers a family with a main parent user.
///
/// This action is used to start using the service. It will ensure that a family
/// with the user specified as parent exists.
#[post("family")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    req: web::Json<Req>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    {
        let response = execute(&mut trans, &req.into_inner()).await?;
        trans.commit().await?;
        Ok(response)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    req: &Req,
) -> Result<Res, api::Error> {
    let family = api::argument(req.family.clone().entity(UID::new()))?;
    let user = api::argument(
        req.user
            .clone()
            .merge(user::Description {
                family_uid: Some(family.uid().clone()),
                role: Some(Role::Parent),
                ..Default::default()
            })
            .entity(UID::new()),
    )?;
    let password = Password::new(
        user.uid().clone(),
        api::argument(PasswordHash::from_password(&req.password))?,
    );

    family.create(&mut *e).await?;
    user.create(&mut *e).await?;
    password.create(&mut *e).await?;

    Ok(Res { family, user })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The family description.
    pub family: family::Description,

    /// The user description.
    pub user: user::Description,

    /// The user password.
    pub password: String,
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct Res {
    /// The generated family.
    pub family: Family,

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
    use crate::db::entities::{Family, User};
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();

        let mut trans = c.begin().await.unwrap();
        let res = execute(
            &mut trans,
            &Req {
                family: family::Description {
                    name: Some("family name".into()),
                    ..Default::default()
                },
                user: user::Description {
                    name: Some("user name".into()),
                    email: Some(None),
                    ..Default::default()
                },
                password: "password".into(),
            },
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(res.family.name(), "family name");
        assert_eq!(res.user.name(), "user name");
        assert_eq!(
            Family::read(&mut c, res.family.uid()).await.unwrap(),
            Some(res.family),
        );
        assert_eq!(
            User::read(&mut c, res.user.uid()).await.unwrap(),
            Some(res.user),
        );
    }
}
