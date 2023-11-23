use crate::prelude::*;

use weru::email::{template::Language, Sender};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::configuration::Configuration;
use crate::db::entities::{invitation, Family, Invitation, User};
use crate::db::values::{Role, Timestamp, UID};

/// The name of the template used for invitations.
const TEMPLATE: &'static str = &"invitation";

/// Invites a new member to a family.
///
/// This action is used to add both parents and children.
#[post("invitation")]
pub async fn handle(
    engine: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    email_sender: web::Data<Box<dyn Sender>>,
    server_configuration: web::Data<Configuration>,
    session: Session,
    req: web::Json<Req>,
) -> impl Responder {
    let mut conn = engine.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let languages = req.language.iter().cloned().collect::<Vec<_>>();
        let res = execute(&mut tx, state.clone(), &req.into_inner()).await?;
        let family = api::expect(
            Family::read(tx.as_mut(), &res.invitation.family_uid).await?,
        )?;
        let invitation_uid = res.invitation.uid.to_string();
        email_sender
            .send(
                api::argument(api::mailbox(
                    &res.invitation.name,
                    &res.invitation.email,
                ))?
                .into(),
                &languages,
                &TEMPLATE.into(),
                &[
                    (String::from("family.name"), family.name.clone()),
                    (String::from("invitation.uid"), invitation_uid.clone()),
                    (
                        String::from("server.url"),
                        server_configuration.server.url.clone(),
                    ),
                ]
                .into_iter()
                .collect(),
            )
            .await?;
        Notify::Family {
            event: Event::FamilyMemberInvited {
                user: res.invitation.user(),
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
) -> Result<Res, api::Error> {
    let state = state.assert_role(Role::Parent)?;
    let invitation = api::argument(
        if req.user.role == Some(Role::Parent) {
            req.user.clone().merge(invitation::InvitationDescription {
                time: Some(Timestamp::now()),
                family_uid: Some(state.family_uid.clone()),
                allowance_amount: Some(None),
                allowance_schedule: Some(None),
                ..Default::default()
            })
        } else {
            req.user.clone().merge(invitation::InvitationDescription {
                time: Some(Timestamp::now()),
                family_uid: Some(state.family_uid.clone()),
                ..Default::default()
            })
        }
        .entity(UID::new()),
    )?;

    if User::read_by_family(&mut *tx, &state.family_uid)
        .await?
        .iter()
        .any(|u| u.name == invitation.name)
    {
        return Err(api::Error::Static(
            StatusCode::CONFLICT,
            "user already exists",
        ));
    }

    invitation.create(tx.as_mut()).await?;

    Ok(Res { invitation })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The invitation.
    pub user: invitation::InvitationDescription,

    /// The language to use for the invitation email.
    pub language: Option<Language>,
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// A description of the future user.
    pub invitation: Invitation,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::invitation;
    use crate::db::test_engine;

    use super::*;

    #[actix_rt::test]
    async fn success_child() {
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
                    user: invitation::InvitationDescription {
                        role: Some(Role::Child),
                        name: Some("child".into()),
                        email: Some("new@test.com".parse().unwrap()),
                        allowance_amount: Some(Some(42)),
                        allowance_schedule: Some(Some("mon".parse().unwrap())),
                        ..Default::default()
                    },
                    language: None,
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        let invitation = Invitation::read(conn.as_mut(), &res.invitation.uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(invitation, res.invitation);
        assert_eq!(invitation.role, Role::Child);
        assert_eq!(invitation.name, "child");
        assert_eq!(invitation.email, "new@test.com".parse().unwrap());
        assert_eq!(invitation.allowance_amount, Some(42));
        assert_eq!(invitation.allowance_schedule, Some("mon".parse().unwrap()));
    }

    #[actix_rt::test]
    async fn success_parent() {
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
                    user: invitation::InvitationDescription {
                        role: Some(Role::Parent),
                        name: Some("parent".into()),
                        email: Some("new@test.com".parse().unwrap()),
                        ..Default::default()
                    },
                    language: None,
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        let invitation = Invitation::read(conn.as_mut(), &res.invitation.uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(invitation, res.invitation);
        assert_eq!(invitation.role, Role::Parent);
        assert_eq!(invitation.name, "parent");
        assert_eq!(invitation.email, "new@test.com".parse().unwrap());
        assert_eq!(invitation.allowance_amount, None);
        assert_eq!(invitation.allowance_schedule, None);
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
                    user: invitation::InvitationDescription {
                        role: Some(Role::Child),
                        name: Some("child".into()),
                        email: Some("new@test.com".parse().unwrap()),
                        allowance_amount: Some(Some(42)),
                        allowance_schedule: Some(Some("mon".parse().unwrap())),
                        ..Default::default()
                    },
                    language: None,
                },
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
