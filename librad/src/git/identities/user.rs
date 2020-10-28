// This file is part of radicle-link
// <https://github.com/radicle-dev/radicle-link>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{
    convert::{TryFrom, TryInto},
    path::Path,
};

use radicle_git_ext::is_not_found_err;

use super::{common, error::Error};
use crate::{
    git::{
        storage2::{self, Storage},
        types::Reference,
    },
    identities::{
        self,
        delegation,
        git::{Identities, User, VerifiedUser, Verifying},
        urn,
    },
    peer::PeerId,
    signer::Signer,
};

pub use identities::{git::Urn, payload::UserPayload};

pub fn get<S>(storage: &Storage<S>, urn: &Urn) -> Result<Option<User>, Error>
where
    S: Signer,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    match storage.reference(&Reference::try_from(urn)?) {
        Ok(Some(reference)) => {
            let tip = reference.peel_to_commit()?.id();
            Ok(Some(identities(storage).get(tip)?))
        },

        Ok(None) => Ok(None),
        Err(storage2::Error::Git(e)) if is_not_found_err(&e) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn verify<S>(storage: &Storage<S>, urn: &Urn) -> Result<Option<VerifiedUser>, Error>
where
    S: Signer,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    match storage.reference(&Reference::try_from(urn)?) {
        Ok(Some(reference)) => {
            let tip = reference.peel_to_commit()?.id();
            identities(storage)
                .verify(tip)
                .map(Some)
                .map_err(|e| Error::Verify(e.into()))
        },

        Ok(None) => Ok(None),
        Err(storage2::Error::Git(e)) if is_not_found_err(&e) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create<S>(
    storage: &Storage<S>,
    payload: impl Into<UserPayload>,
    delegations: delegation::Direct,
) -> Result<User, Error>
where
    S: Signer,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    let user = identities(storage).create(payload.into(), delegations, storage.signer())?;
    let urn = user.urn();

    common::IdRef::from(&urn).create(storage, user.content_id)?;

    Ok(user)
}

pub fn update<S>(
    storage: &Storage<S>,
    urn: &Urn,
    payload: impl Into<Option<UserPayload>>,
    delegations: impl Into<Option<delegation::Direct>>,
) -> Result<User, Error>
where
    S: Signer,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    let prev = get(storage, urn)?.ok_or_else(|| Error::NotFound(urn.clone()))?;
    let prev = Verifying::from(prev).signed()?;
    let next = identities(storage).update(prev, payload, delegations, storage.signer())?;

    common::IdRef::from(urn).update(storage, next.content_id, "update")?;

    Ok(next)
}

pub fn merge<S>(storage: &Storage<S>, urn: &Urn, from: PeerId) -> Result<User, Error>
where
    S: Signer,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    let ours = get(storage, urn)?.ok_or_else(|| Error::NotFound(urn.clone()))?;
    let theirs = {
        let their_urn = Urn {
            id: urn.id,
            path: Some(
                Path::new("remotes")
                    .join(from.to_string())
                    .join(&*urn::DEFAULT_PATH)
                    .try_into()
                    .unwrap(),
            ),
        };
        get(storage, &their_urn)?.ok_or_else(|| Error::NotFound(their_urn))?
    };

    let ours = Verifying::from(ours).signed()?;
    let theirs = Verifying::from(theirs).signed()?;
    let next = identities(storage).update_from(ours, theirs, storage.signer())?;

    common::IdRef::from(urn).update(storage, next.content_id, &format!("merge from {}", from))?;

    Ok(next)
}

fn identities<S>(storage: &Storage<S>) -> Identities<User>
where
    S: Signer,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    storage.identities()
}
