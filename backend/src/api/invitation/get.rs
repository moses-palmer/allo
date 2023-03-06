use sqlx::prelude::*;

use actix_web::{get, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::db;
use crate::db::entities::{Entity, Family, Invitation};
use crate::db::values::UID;

/// Retrieves information about an invitation.
#[get("invitation/{invitation_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    path: web::Path<UID>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let invitation_uid = path.into_inner();
    {
        let res = execute(&mut trans, &invitation_uid).await?;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    invitation_uid: &UID,
) -> Result<Res, api::Error> {
    let invitation =
        api::expect(Invitation::read(&mut *trans, invitation_uid).await?)?;
    let family =
        api::expect(Family::read(&mut *trans, invitation.family_uid()).await?)?;

    Ok(Res { invitation, family })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The invitation.
    invitation: Invitation,

    /// The family.
    family: Family,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_pool;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success() {
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

        let res = execute(&mut pool.begin().await.unwrap(), invitation.uid())
            .await
            .unwrap();

        assert_eq!(res.invitation, invitation);
        assert_eq!(res.family, family);
    }

    #[actix_rt::test]
    async fn not_found() {
        let pool = test_pool().await;

        let err = execute(&mut pool.begin().await.unwrap(), &UID::new())
            .await
            .err()
            .unwrap();

        assert_eq!(err, api::Error::not_found("not found"));
    }
}
