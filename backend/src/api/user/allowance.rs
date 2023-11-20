use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::{allowance, Allowance, User};
use crate::db::values::{Role, UID};

/// Changes the allowance for a user.
#[put("user/{user_uid}/allowance/{allowance_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    req: web::Json<Req>,
    path: web::Path<(UID, UID)>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    let (user_uid, allowance_uid) = path.into_inner();
    {
        let res = execute(
            &mut tx,
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
        .send(&mut tx, &channel, &state.user_uid)
        .await;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    state: State,
    req: &Req,
    user_uid: &UID,
    allowance_uid: &UID,
) -> Result<Res, api::Error> {
    let user = api::expect(User::read(tx.as_mut(), user_uid).await?)?;
    state
        .assert_family(&user.family_uid)?
        .assert_role(Role::Parent)?;

    let allowance =
        api::expect(Allowance::read(tx.as_mut(), &allowance_uid).await?)?
            .merge(req.clone().merge(allowance::AllowanceDescription {
                user_uid: Some(user.uid.clone()),
                ..Default::default()
            }));
    allowance.update(tx.as_mut()).await?;

    Ok(Res { allowance })
}

pub type Req = allowance::AllowanceDescription;

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The new allowance.
    allowance: Allowance,
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
        let allowance = create::allowance(
            &mut conn,
            &children.0.uid,
            42,
            "mon".parse().unwrap(),
        );

        {
            let mut tx = conn.begin().await.unwrap();
            execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &Req {
                    amount: Some(84),
                    schedule: Some("tue".parse().unwrap()),
                    ..Default::default()
                },
                &allowance.user_uid,
                &allowance.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        let allowance = Allowance::read(conn.as_mut(), &allowance.uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(allowance.user_uid, children.0.uid);
        assert_eq!(allowance.amount, 84);
        assert_eq!(allowance.schedule, "tue".parse().unwrap());
    }

    #[actix_rt::test]
    async fn success_user_uid_ignored() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, children, _, _) =
            tests::populate(&mut conn).unwrap();
        let allowance = create::allowance(
            &mut conn,
            &children.0.uid,
            42,
            "mon".parse().unwrap(),
        );

        {
            let mut tx = conn.begin().await.unwrap();
            execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &Req {
                    user_uid: Some(children.1.uid.clone()),
                    amount: Some(84),
                    schedule: Some("tue".parse().unwrap()),
                    ..Default::default()
                },
                &allowance.user_uid,
                &allowance.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        let allowance = Allowance::read(conn.as_mut(), &allowance.uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(allowance.user_uid, children.0.uid);
        assert_eq!(allowance.amount, 84);
        assert_eq!(allowance.schedule, "tue".parse().unwrap());
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut conn).unwrap();
        let other_family = create::family(&mut conn, "Other Family");
        let other_child = create::user(
            &mut conn,
            Role::Child,
            "Other User",
            "other@email.com",
            &other_family.uid,
        );
        let allowance = create::allowance(
            &mut conn,
            &other_child.uid,
            42,
            "mon".parse().unwrap(),
        );

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: children.0.role.clone(),
                },
                &Req {
                    amount: Some(84),
                    schedule: Some("tue".parse().unwrap()),
                    ..Default::default()
                },
                &allowance.user_uid,
                &allowance.uid,
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
        let allowance = create::allowance(
            &mut conn,
            &children.0.uid,
            42,
            "mon".parse().unwrap(),
        );

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: children.0.role.clone(),
                },
                &Req {
                    amount: Some(84),
                    schedule: Some("tue".parse().unwrap()),
                    ..Default::default()
                },
                &allowance.user_uid,
                &allowance.uid,
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
