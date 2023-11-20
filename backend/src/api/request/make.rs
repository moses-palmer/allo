use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::{request, Request};
use crate::db::values::{Role, Timestamp, UID};

/// Generates a user request.
#[post("request/{user_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    req: web::Json<Req>,
    user_uid: web::Path<UID>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(
            &mut tx,
            state.clone(),
            &req.into_inner(),
            &user_uid.into_inner(),
        )
        .await?;
        Notify::Parents {
            event: Event::RequestCreated {
                request: res.request.clone(),
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
    req: &Req,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    state.assert_user(&user_uid)?.assert_role(Role::Child)?;

    let request = Request::create_with_auto_uid(
        tx,
        user_uid.clone(),
        api::argument(req.name.clone())?,
        api::argument(req.description.clone())?,
        api::argument(req.amount)?,
        req.url.clone().flatten(),
        Timestamp::now(),
    )
    .await?;

    Ok(Res { request })
}

pub type Req = request::RequestDescription;

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The generated request.
    pub request: Request,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::test_engine;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut conn).unwrap();
        let amount = 424242;

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    role: children.0.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Req {
                    name: Some("A name".into()),
                    description: Some("A description!".into()),
                    amount: Some(amount),
                    url: Some(None),
                    ..Default::default()
                },
                &children.0.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.request.amount, amount);
        assert_eq!(
            Request::read(conn.as_mut(), &res.request.uid)
                .await
                .unwrap(),
            Some(res.request),
        );
    }

    #[actix_rt::test]
    async fn forbidden() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut conn).unwrap();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.1.uid.clone(),
                    role: children.1.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Req {
                    name: Some("A name".into()),
                    description: Some("A description!".into()),
                    amount: Some(0),
                    url: Some(None),
                    ..Default::default()
                },
                &children.0.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }
}
