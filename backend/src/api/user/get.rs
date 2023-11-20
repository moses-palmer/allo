use crate::prelude::*;

use crate::api;
use crate::api::session::State;
use crate::db::entities::{Allowance, User};
use crate::db::values::UID;

/// Retrieves information about a user.
#[get("user/{user_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    session: Session,
    path: web::Path<UID>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    let user_uid = path.into_inner();
    {
        let res = execute(&mut tx, state, &user_uid).await?;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    state: State,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = api::expect(User::read(tx.as_mut(), user_uid).await?)?;
    state.assert_family(&user.family_uid)?;
    let allowance = Allowance::read_for_user(tx, &user.uid)
        .await?
        .into_iter()
        .next();

    Ok(Res { user, allowance })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The user.
    user: User,

    /// The allowance schedule.
    allowance: Option<Allowance>,
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
        let (family, parent, children, _, _) =
            tests::populate(&mut conn).unwrap();
        let allowance = create::allowance(
            &mut conn,
            &children.0.uid,
            43,
            "mon".parse().unwrap(),
        );

        let mut tx = conn.begin().await.unwrap();
        let res = execute(
            &mut tx,
            State {
                user_uid: parent.uid.clone(),
                role: parent.role.clone(),
                family_uid: family.uid.clone(),
            },
            &children.0.uid,
        )
        .await
        .unwrap();

        assert_eq!(res.user, children.0);
        assert_eq!(res.allowance, Some(allowance));
    }

    #[actix_rt::test]
    async fn success_parent_self() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();

        let mut tx = conn.begin().await.unwrap();
        let res = execute(
            &mut tx,
            State {
                user_uid: parent.uid.clone(),
                role: parent.role.clone(),
                family_uid: family.uid.clone(),
            },
            &parent.uid,
        )
        .await
        .unwrap();

        assert_eq!(res.user, parent);
        assert_eq!(res.allowance, None);
    }

    #[actix_rt::test]
    async fn success_child() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut conn).unwrap();
        let allowance = create::allowance(
            &mut conn,
            &children.0.uid,
            43,
            "mon".parse().unwrap(),
        );

        let mut tx = conn.begin().await.unwrap();
        let res = execute(
            &mut tx,
            State {
                user_uid: children.0.uid.clone(),
                role: children.0.role.clone(),
                family_uid: family.uid.clone(),
            },
            &children.0.uid,
        )
        .await
        .unwrap();

        assert_eq!(res.user, children.0);
        assert_eq!(res.allowance, Some(allowance));
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

        let mut tx = conn.begin().await.unwrap();
        let err = execute(
            &mut tx,
            State {
                user_uid: parent.uid.clone(),
                family_uid: family.uid.clone(),
                role: parent.role.clone(),
            },
            &other_child.uid,
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }
}
