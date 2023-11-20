use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::db::entities::{allowance, Invitation, Password, User};
use crate::db::values::{PasswordHash, UID};

/// Responds to an invitation to join a family.
///
/// This action is used by both parents and children.
#[post("invitation/{invitation_uid}/accept")]
pub async fn handle(
    engine: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    req: web::Json<Req>,
    path: web::Path<UID>,
) -> impl Responder {
    let mut conn = engine.connection().await?;
    let mut tx = conn.begin().await?;
    let invitation_uid = path.into_inner();
    {
        let res = execute(&mut tx, &req.into_inner(), &invitation_uid).await?;
        Notify::Family {
            event: Event::FamilyMemberAdded {
                user: res.user.clone(),
                by: res.user.uid.clone(),
            },
            family: res.user.family_uid.clone(),
        }
        .send(&mut tx, &channel, &res.user.uid)
        .await;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    req: &Req,
    invitation_uid: &UID,
) -> Result<Res, api::Error> {
    let invitation =
        api::expect(Invitation::read(tx.as_mut(), invitation_uid).await?)?;
    let user = api::expect(invitation.user().entity(UID::new()))?;
    let password = Password::new(
        user.uid.clone(),
        api::argument(PasswordHash::from_password(&req.password))?,
    );

    if User::read_for_family(tx, &invitation.family_uid)
        .await?
        .iter()
        .any(|u| u.name == user.name)
    {
        return Err(api::Error::Static(
            StatusCode::CONFLICT,
            "user already exists",
        ));
    }

    user.create(tx.as_mut()).await?;
    password.create(tx.as_mut()).await?;
    if let Some(allowance) = invitation.allowance() {
        api::argument(
            allowance
                .merge(allowance::AllowanceDescription {
                    user_uid: Some(user.uid.clone()),
                    ..Default::default()
                })
                .entity(UID::new()),
        )?
        .create(tx.as_mut())
        .await?;
    }
    invitation.delete(tx.as_mut()).await?;

    Ok(Res { user })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The user password.
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The generated user.
    pub user: User,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::entities::Allowance;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, _, _, _) = tests::populate(&mut conn).unwrap();
        let invitation = create::invitation(
            &mut conn,
            Role::Parent,
            "New User",
            "new@test.com",
            &family.uid,
        );

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    password: "123".into(),
                },
                &invitation.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        let user = User::read(conn.as_mut(), &res.user.uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user, res.user);
        assert_eq!(user.role, Role::Parent);
        assert_eq!(user.name, "New User");
        assert_eq!(user.email, Some("new@test.com".parse().unwrap()));

        let password = Password::read(conn.as_mut(), &user.uid)
            .await
            .unwrap()
            .unwrap();
        assert!(password.hash.verify("123").unwrap());

        let mut tx = conn.begin().await.unwrap();
        let allowances =
            Allowance::read_for_user(&mut tx, &user.uid).await.unwrap();
        assert_eq!(allowances.len(), 0);
    }

    #[actix_rt::test]
    async fn success_child() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, _, _, _) = tests::populate(&mut conn).unwrap();
        let invitation = create::invitation(
            &mut conn,
            Role::Child,
            "New User",
            "new@test.com",
            &family.uid,
        );

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    password: "123".into(),
                },
                &invitation.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        let user = User::read(conn.as_mut(), &res.user.uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user, res.user);
        assert_eq!(user.role, Role::Child);
        assert_eq!(user.name, "New User");
        assert_eq!(user.email, Some("new@test.com".parse().unwrap()));

        let password = Password::read(conn.as_mut(), &user.uid)
            .await
            .unwrap()
            .unwrap();
        assert!(password.hash.verify("123").unwrap());

        let mut tx = conn.begin().await.unwrap();
        let allowances =
            Allowance::read_for_user(&mut tx, &user.uid).await.unwrap();
        assert_eq!(allowances.len(), 1);
        assert_eq!(allowances[0].amount, 42);
        assert_eq!(allowances[0].schedule, "mon".parse().unwrap());
    }

    #[actix_rt::test]
    async fn conflict_name() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, (child, _), _, _) = tests::populate(&mut conn).unwrap();
        let invitation = create::invitation(
            &mut conn,
            Role::Parent,
            &child.name,
            "new@test.com",
            &family.uid,
        );

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    password: "123".into(),
                },
                &invitation.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(
            err,
            api::Error::Static(StatusCode::CONFLICT, "user already exists")
        );
    }
}
