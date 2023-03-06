use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::configuration::Configuration;
use crate::db;
use crate::db::entities::{invitation, Entity, Family, Invitation, User};
use crate::db::values::{Role, Timestamp, UID};
use crate::email::template::Language;
use crate::email::{mailbox, Sender, Transport};
use crate::notifications::Notifier;

/// The name of the template used for invitations.
const TEMPLATE: &'static str = &"invitation";

/// Invites a new member to a family.
///
/// This action is used to add both parents and children.
#[post("invitation")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    email_sender: web::Data<Arc<Sender<Transport>>>,
    server_configuration: web::Data<Arc<Configuration>>,
    session: Session,
    req: web::Json<Req>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    {
        let language = req.language.clone();
        let res = execute(&mut trans, state.clone(), &req.into_inner()).await?;
        let family = api::expect(
            Family::read(&mut trans, res.invitation.family_uid()).await?,
        )?;
        let invitation_uid = res.invitation.uid().to_string();
        email_sender
            .send(
                api::argument(mailbox(
                    res.invitation.name(),
                    res.invitation.email(),
                ))?,
                vec![
                    language.unwrap_or_else(|| {
                        server_configuration.email.default_language.clone()
                    }),
                    server_configuration.email.default_language.clone(),
                ],
                &TEMPLATE.into(),
                |key| match key {
                    "family.name" => Some(family.name()),
                    "invitation.uid" => Some(&invitation_uid),
                    "server.url" => Some(&server_configuration.server.url),
                    _ => None,
                },
            )
            .await?;
        Notify::Family {
            event: Event::FamilyMemberInvited {
                user: res.invitation.user(),
                by: state.user_uid.clone(),
            },
            family: state.family_uid,
        }
        .send(&mut *trans, &notifier, &state.user_uid)
        .await;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    state: State,
    req: &Req,
) -> Result<Res, api::Error> {
    let state = state.assert_role(Role::Parent)?;
    let invitation = api::argument(
        if req.user.role == Some(Role::Parent) {
            req.user.clone().merge(invitation::Description {
                time: Some(Timestamp::now()),
                family_uid: Some(state.family_uid.clone()),
                allowance_amount: Some(None),
                allowance_schedule: Some(None),
                ..Default::default()
            })
        } else {
            req.user.clone().merge(invitation::Description {
                time: Some(Timestamp::now()),
                family_uid: Some(state.family_uid.clone()),
                ..Default::default()
            })
        }
        .entity(UID::new()),
    )?;

    if User::read_for_family(&mut *trans, &state.family_uid)
        .await?
        .iter()
        .any(|u| u.name() == invitation.name())
    {
        return Err(api::Error::Static(
            StatusCode::CONFLICT,
            "user already exists",
        ));
    }

    invitation.create(&mut *trans).await?;

    Ok(Res { invitation })
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The invitation.
    pub user: invitation::Description,

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
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                user: invitation::Description {
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
        trans.commit().await.unwrap();

        let invitation = Invitation::read(&mut c, res.invitation.uid())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(invitation, res.invitation);
        assert_eq!(invitation.role(), &Role::Child);
        assert_eq!(invitation.name(), "child");
        assert_eq!(invitation.email(), &"new@test.com".parse().unwrap());
        assert_eq!(invitation.allowance_amount(), &Some(42));
        assert_eq!(
            invitation.allowance_schedule(),
            &Some("mon".parse().unwrap())
        );
    }

    #[actix_rt::test]
    async fn success_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                user: invitation::Description {
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
        trans.commit().await.unwrap();

        let invitation = Invitation::read(&mut c, res.invitation.uid())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(invitation, res.invitation);
        assert_eq!(invitation.role(), &Role::Parent);
        assert_eq!(invitation.name(), "parent");
        assert_eq!(invitation.email(), &"new@test.com".parse().unwrap());
        assert_eq!(invitation.allowance_amount(), &None);
        assert_eq!(invitation.allowance_schedule(), &None,);
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: children.0.uid().clone(),
                family_uid: family.uid().clone(),
                role: children.0.role().clone(),
            },
            &Req {
                user: invitation::Description {
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
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
