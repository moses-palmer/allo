use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::{Request, Transaction, User};
use crate::db::values::{Role, Timestamp, TransactionType, UID};

/// Grants a request.
#[post("request/{user_uid}/{request_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    req: web::Json<Req>,
    path: web::Path<(UID, i64)>,
) -> impl Responder {
    let mut connection = database.connection().await?;
    let mut tx = connection.begin().await?;
    let state = State::load(&session)?;
    let (user_uid, request_uid) = path.into_inner();
    {
        let res = execute(
            &mut tx,
            state.clone(),
            &req.into_inner(),
            &user_uid,
            &request_uid,
        )
        .await?;
        Notify::MemberAndParents {
            event: Event::RequestGranted {
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
    req: &Req,
    user_uid: &UID,
    request_uid: &i64,
) -> Result<Res, api::Error> {
    let request = Request::read(tx.as_mut(), request_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    let user = User::read(tx.as_mut(), user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    state
        .assert_role(Role::Parent)?
        .assert_family(&user.family_uid)?;

    if request.user_uid != user.uid {
        Err(api::Error::not_found("unknown request"))
    } else {
        let transaction = Transaction::create_with_auto_uid(
            tx,
            TransactionType::Request,
            user.uid.clone(),
            request.name.clone(),
            -(req.cost.unwrap_or(request.amount) as i64),
            Timestamp::now(),
        )
        .await?;
        request.delete(tx.as_mut()).await?;
        Ok(Res {
            request,
            transaction,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The actual, reviewed cost of this item.
    ///
    /// If this is not present, the value from the database is used.
    pub cost: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The request that was granted.
    pub request: Request,

    /// The generated transaction.
    pub transaction: Transaction,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_no_cost() {
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
                &Req { cost: None },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.transaction.amount, -candidate.amount);
        assert!(Request::read(conn.as_mut(), &candidate.uid)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            Transaction::read(conn.as_mut(), &res.transaction.uid)
                .await
                .unwrap(),
            Some(res.transaction),
        );
    }

    #[actix_rt::test]
    async fn success_with_cost() {
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
        let cost = 12345678;

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Req { cost: Some(cost) },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.transaction.amount, -cost);
        assert!(Request::read(conn.as_mut(), &candidate.uid)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            Transaction::read(conn.as_mut(), &res.transaction.uid)
                .await
                .unwrap(),
            Some(res.transaction),
        );
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
                &Req { cost: None },
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
                &Req { cost: None },
                &children.0.uid,
                &candidate.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid role"));
        assert_eq!(
            Request::read(conn.as_mut(), &candidate.uid).await.unwrap(),
            Some(candidate),
        );
    }
}
