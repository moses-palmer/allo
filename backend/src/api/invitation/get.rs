use crate::prelude::*;

use crate::api;
use crate::db::entities::{Family, Invitation};
use crate::db::values::UID;

/// Retrieves information about an invitation.
#[get("invitation/{invitation_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    path: web::Path<UID>,
) -> impl Responder {
    let mut connection = database.connection().await?;
    let mut tx = connection.begin().await?;
    let invitation_uid = path.into_inner();
    {
        let res = execute(&mut tx, &invitation_uid).await?;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    invitation_uid: &UID,
) -> Result<Res, api::Error> {
    let invitation =
        api::expect(Invitation::read(tx.as_mut(), invitation_uid).await?)?;
    let family =
        api::expect(Family::read(tx.as_mut(), &invitation.family_uid).await?)?;

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
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, _, _, _) = tests::populate(&mut conn).unwrap();
        let invitation = create::invitation(
            &mut conn,
            Role::Parent,
            "New User",
            "new@test.com",
            &family.uid,
        );

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(&mut tx, &invitation.uid).await.unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.invitation, invitation);
        assert_eq!(res.family, family);
    }

    #[actix_rt::test]
    async fn not_found() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(&mut tx, &UID::new()).await.err().unwrap();
            r
        };

        assert_eq!(err, api::Error::not_found("not found"));
    }
}
