use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::User;
use crate::db::values::{Role, UID};

/// Removes a member from a family.
#[delete("family/{family_uid}/{user_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    path: web::Path<(UID, UID)>,
) -> impl Responder {
    let mut connection = database.connection().await?;
    let mut tx = connection.begin().await?;
    let state = State::load(&session)?;
    let (family_uid, user_uid) = path.into_inner();
    {
        let res =
            execute(&mut tx, state.clone(), &family_uid, &user_uid).await?;
        Notify::Family {
            event: Event::FamilyMemberRemoved {
                user: res.user.clone(),
                by: state.user_uid.clone(),
            },
            family: state.family_uid,
        }
        .send(&mut tx, &channel, &state.user_uid)
        .await;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    state: State,
    family_uid: &UID,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = User::read(tx.as_mut(), user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown user"))?;
    let state = state
        .assert_role(Role::Parent)?
        .assert_family(family_uid)?
        .assert_family(&user.family_uid)?;
    if state.user_uid == user.uid {
        Err(api::Error::forbidden("cannot remove self"))
    } else {
        user.delete(tx.as_mut()).await?;
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
    use crate::db::test_engine;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, children, _, _) =
            tests::populate(&mut conn).unwrap();

        {
            let mut tx = conn.begin().await.unwrap();
            execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &children.0.family_uid,
                &children.0.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        assert!(User::read(conn.as_mut(), &children.0.uid)
            .await
            .unwrap()
            .is_none());
    }

    #[actix_rt::test]
    async fn forbidden_parent_self() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &parent.family_uid,
                &parent.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("cannot remove self"));
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();
        let other_family = create::family(&mut conn, "Other Family");
        let other_child = create::user(
            &mut conn,
            Role::Child,
            "Other User",
            "other@email.com",
            &other_family.uid,
        );

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &other_child.family_uid,
                &other_child.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut conn).unwrap();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: children.0.role.clone(),
                },
                &children.1.family_uid,
                &children.1.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
