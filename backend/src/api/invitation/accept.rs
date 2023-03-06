use sqlx::prelude::*;

use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::db;
use crate::db::entities::{allowance, Entity, Invitation, Password, User};
use crate::db::values::{PasswordHash, UID};
use crate::notifications::Notifier;

/// Responds to an invitation to join a family.
///
/// This action is used by both parents and children.
#[post("invitation/{invitation_uid}/accept")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    req: web::Json<Req>,
    path: web::Path<UID>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let invitation_uid = path.into_inner();
    {
        let res =
            execute(&mut trans, &req.into_inner(), &invitation_uid).await?;
        Notify::Family {
            event: Event::FamilyMemberAdded {
                user: res.user.clone(),
                by: res.user.uid().clone(),
            },
            family: res.user.family_uid().clone(),
        }
        .send(&mut *trans, &notifier, res.user.uid())
        .await;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    req: &Req,
    invitation_uid: &UID,
) -> Result<Res, api::Error> {
    let invitation =
        api::expect(Invitation::read(&mut *trans, invitation_uid).await?)?;
    let user = api::expect(invitation.user().entity(UID::new()))?;
    let password = Password::new(
        user.uid().clone(),
        api::argument(PasswordHash::from_password(&req.password))?,
    );

    if User::read_for_family(&mut *trans, invitation.family_uid())
        .await?
        .iter()
        .any(|u| u.name() == user.name())
    {
        return Err(api::Error::Static(
            StatusCode::CONFLICT,
            "user already exists",
        ));
    }

    user.create(&mut *trans).await?;
    password.create(&mut *trans).await?;
    if let Some(allowance) = invitation.allowance() {
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
    invitation.delete(&mut *trans).await?;

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
    use crate::db::test_pool;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, _, _, _) = tests::populate(&mut c).unwrap();
        let invitation = create::invitation(
            &mut c,
            Role::Parent,
            "New User",
            "new@test.com",
            family.uid(),
        );

        let mut trans = c.begin().await.unwrap();
        let res = execute(
            &mut trans,
            &Req {
                password: "123".into(),
            },
            invitation.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        let user = User::read(&mut c, res.user.uid()).await.unwrap().unwrap();
        assert_eq!(user, res.user);
        assert_eq!(user.role(), &Role::Parent);
        assert_eq!(user.name(), "New User");
        assert_eq!(user.email(), &Some("new@test.com".parse().unwrap()));

        let password =
            Password::read(&mut c, user.uid()).await.unwrap().unwrap();
        assert!(password.hash().verify("123").unwrap());

        let allowances =
            Allowance::read_for_user(&mut c, user.uid()).await.unwrap();
        assert_eq!(allowances.len(), 0);
    }

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, _, _, _) = tests::populate(&mut c).unwrap();
        let invitation = create::invitation(
            &mut c,
            Role::Child,
            "New User",
            "new@test.com",
            family.uid(),
        );

        let mut trans = c.begin().await.unwrap();
        let res = execute(
            &mut trans,
            &Req {
                password: "123".into(),
            },
            invitation.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        let user = User::read(&mut c, res.user.uid()).await.unwrap().unwrap();
        assert_eq!(user, res.user);
        assert_eq!(user.role(), &Role::Child);
        assert_eq!(user.name(), "New User");
        assert_eq!(user.email(), &Some("new@test.com".parse().unwrap()));

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
    async fn conflict_name() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, (child, _), _, _) = tests::populate(&mut c).unwrap();
        let invitation = create::invitation(
            &mut c,
            Role::Parent,
            child.name(),
            "new@test.com",
            family.uid(),
        );

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            &Req {
                password: "123".into(),
            },
            invitation.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(
            err,
            api::Error::Static(StatusCode::CONFLICT, "user already exists")
        );
    }
}
