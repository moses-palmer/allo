use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::{Request, User};
use crate::db::values::{Role, UID};

/// Deletes a user request
#[delete("request/{user_uid}/{request_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    path: web::Path<(UID, i64)>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    let (user_uid, request_uid) = path.into_inner();
    {
        let res =
            execute(&mut tx, state.clone(), &user_uid, &request_uid).await?;
        Notify::MemberAndParents {
            event: Event::RequestDeclined {
                request: res.request.clone(),
                by: state.user_uid.clone(),
            },
            uid: user_uid,
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
    user_uid: &UID,
    request_uid: &i64,
) -> Result<Res, api::Error> {
    let request = Request::read(tx.as_mut(), request_uid)
        .await?
        .filter(|request| &request.user_uid == user_uid)
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    let user = User::read(tx.as_mut(), user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    match state.role {
        Role::Parent => state.assert_family(&user.family_uid)?,
        Role::Child => state.assert_user(&user.uid)?,
    };

    request.delete(tx.as_mut()).await?;
    Ok(Res { request })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The request that was declined.
    request: Request,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, children, _, requests) =
            tests::populate(&mut conn).unwrap();
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid == children.0.uid)
            .next()
            .unwrap()
            .clone();

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.request, candidate);
        assert!(Request::read(conn.as_mut(), &candidate.uid)
            .await
            .unwrap()
            .is_none());
    }

    #[actix_rt::test]
    async fn success_child() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, requests) =
            tests::populate(&mut conn).unwrap();
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid == children.0.uid)
            .next()
            .unwrap()
            .clone();

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    role: children.0.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.request, candidate);
        assert!(Request::read(conn.as_mut(), &candidate.uid)
            .await
            .unwrap()
            .is_none());
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (_, _, children, _, requests) = tests::populate(&mut conn).unwrap();
        let other_family = create::family(&mut conn, "Other Family");
        let other_parent = create::user(
            &mut conn,
            Role::Parent,
            "Other User",
            "other@email.com",
            &other_family.uid,
        );
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid == children.0.uid)
            .next()
            .unwrap()
            .clone();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: other_parent.uid.clone(),
                    role: other_parent.role.clone(),
                    family_uid: other_family.uid.clone(),
                },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid family"));
        assert_eq!(
            Request::read(conn.as_mut(), &candidate.uid).await.unwrap(),
            Some(candidate),
        );
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, requests) =
            tests::populate(&mut conn).unwrap();
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid == children.0.uid)
            .next()
            .unwrap()
            .clone();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.1.uid.clone(),
                    role: children.1.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid user"));
        assert_eq!(
            Request::read(conn.as_mut(), &candidate.uid).await.unwrap(),
            Some(candidate),
        );
    }
}
