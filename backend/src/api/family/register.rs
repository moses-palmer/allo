use crate::prelude::*;

use crate::api;
use crate::db::entities::{family, user, Family, Password, User};
use crate::db::values::{PasswordHash, Role, UID};

/// Registers a family with a main parent user.
///
/// This action is used to start using the service. It will ensure that a family
/// with the user specified as parent exists.
#[post("family")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    req: web::Json<Req>,
) -> impl Responder {
    let mut connection = database.connection().await?;
    let mut trans = connection.begin().await?;
    {
        let res = execute(&mut trans, &req.into_inner()).await?;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    req: &Req,
) -> Result<Res, api::Error> {
    let family = api::argument(req.family.clone().entity(UID::new()))?;
    let user = api::argument(
        req.user
            .clone()
            .merge(user::UserDescription {
                family_uid: Some(family.uid.clone()),
                role: Some(Role::Parent),
                ..Default::default()
            })
            .entity(UID::new()),
    )?;
    let password = Password::new(
        user.uid.clone(),
        api::argument(PasswordHash::from_password(&req.password))?,
    );

    family.create(tx.as_mut()).await?;
    user.create(tx.as_mut()).await?;
    password.create(tx.as_mut()).await?;

    Ok(Res { family, user })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The family description.
    pub family: family::FamilyDescription,

    /// The user description.
    pub user: user::UserDescription,

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

#[cfg(test)]
mod tests {
    use crate::db::entities::{Family, User};
    use crate::db::test_engine;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    family: family::FamilyDescription {
                        name: Some("family name".into()),
                        ..Default::default()
                    },
                    user: user::UserDescription {
                        name: Some("user name".into()),
                        email: Some(None),
                        ..Default::default()
                    },
                    password: "password".into(),
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.family.name, "family name");
        assert_eq!(res.user.name, "user name");
        assert_eq!(
            Family::read(conn.as_mut(), &res.family.uid).await.unwrap(),
            Some(res.family),
        );
        assert_eq!(
            User::read(conn.as_mut(), &res.user.uid).await.unwrap(),
            Some(res.user),
        );
    }
}
