use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::{allowance, user, Password, User};
use crate::db::values::{PasswordHash, Role, UID};

/// Adds a new member to a family.
///
/// This action is used to add both parents and children.
#[post("family/{family_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    req: web::Json<Req>,
    path: web::Path<UID>,
) -> impl Responder {
    let mut connection = database.connection().await?;
    let mut tx = connection.begin().await?;
    let state = State::load(&session)?;
    let family_uid = path.into_inner();
    {
        let res =
            execute(&mut tx, state.clone(), &req.into_inner(), &family_uid)
                .await?;
        Notify::Family {
            event: Event::FamilyMemberAdded {
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
    req: &Req,
    family_uid: &UID,
) -> Result<Res, api::Error> {
    state
        .assert_role(Role::Parent)?
        .assert_family(&family_uid)?;
    let user = api::argument(
        req.user
            .clone()
            .merge(user::UserDescription {
                family_uid: Some(family_uid.clone()),
                ..Default::default()
            })
            .entity(UID::new()),
    )?;
    if user.role == Role::Parent && req.allowance.is_some() {
        return Err(api::Error::Static(
            StatusCode::BAD_REQUEST,
            "a parent cannot have an allowance",
        ));
    }
    if User::read_for_family(tx, &family_uid)
        .await?
        .iter()
        .any(|u| u.name == user.name)
    {
        return Err(api::Error::Static(
            StatusCode::CONFLICT,
            "user already exists",
        ));
    }
    let password = Password::new(
        user.uid.clone(),
        api::argument(PasswordHash::from_password(&req.password))?,
    );

    user.create(tx.as_mut()).await?;
    password.create(tx.as_mut()).await?;
    if let Some(allowance) = req.allowance.clone() {
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

    Ok(Res { user })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The user to add.
    pub user: user::UserDescription,

    /// The user allowance if they are a child.
    pub allowance: Option<allowance::AllowanceDescription>,

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

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &Req {
                    user: user::UserDescription {
                        role: Some(Role::Child),
                        name: Some("child".into()),
                        email: Some(None),
                        ..Default::default()
                    },
                    allowance: Some(allowance::AllowanceDescription {
                        amount: Some(42),
                        schedule: Some("mon".parse().unwrap()),
                        ..Default::default()
                    }),
                    password: "123".into(),
                },
                &family.uid,
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
        assert_eq!(user.name, "child");
        assert_eq!(user.email, None);

        let password = Password::read(conn.as_mut(), &user.uid)
            .await
            .unwrap()
            .unwrap();
        assert!(password.hash.verify("123").unwrap());

        let allowances = Allowance::read_for_user(
            &mut conn.begin().await.unwrap(),
            &user.uid,
        )
        .await
        .unwrap();
        assert_eq!(allowances.len(), 1);
        assert_eq!(allowances[0].amount, 42);
        assert_eq!(allowances[0].schedule, "mon".parse().unwrap());
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();
        let other_family = create::family(&mut conn, "Other Family");

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &Req {
                    user: user::UserDescription {
                        role: Some(Role::Child),
                        name: Some("child".into()),
                        email: Some(None),
                        ..Default::default()
                    },
                    allowance: None,
                    password: "123".into(),
                },
                &other_family.uid,
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
                &Req {
                    user: user::UserDescription {
                        role: Some(Role::Child),
                        name: Some("child".into()),
                        email: Some(None),
                        ..Default::default()
                    },
                    allowance: None,
                    password: "123".into(),
                },
                &family.uid,
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
