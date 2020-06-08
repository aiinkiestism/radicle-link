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

use std::collections::HashSet;

use thiserror::Error;

use crate::{
    git::{
        refs::Refs,
        storage::{self, Storage},
        types::Namespace,
    },
    internal::borrow::{TryCow, TryToOwned},
    peer::PeerId,
    uri::RadUrn,
};
use radicle_surf::vcs::git as surf;

pub use storage::Tracked;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Storage(#[from] storage::Error),

    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// A logical repository.
///
/// This is just a (thin) wrapper around [`Storage`] so the [`RadUrn`] context
/// doesn't need to be passed around.
pub struct Repo<'a> {
    pub urn: RadUrn,
    pub(super) storage: TryCow<'a, Storage>,
}

impl<'a> Repo<'a> {
    pub fn namespace(&self) -> Namespace {
        self.urn.id.clone()
    }

    /// Fetch new refs and objects for this repo from [`PeerId`]
    pub fn fetch(&self, from: &PeerId) -> Result<(), Error> {
        self.storage
            .fetch_repo(&self.urn, from)
            .map_err(Error::from)
    }

    /// Obtain a read-only view of this repo
    pub fn browser(&'_ self, revision: &str) -> Result<surf::Browser<'_>, Error> {
        self.storage
            .browser(&self.urn, revision)
            .map_err(Error::from)
    }

    /// Track [`PeerId`]s view of this repo
    ///
    /// Equivalent to `git remote add`.
    pub fn track(&self, peer: &PeerId) -> Result<(), Error> {
        self.storage.track(&self.urn, peer).map_err(Error::from)
    }

    /// Stop tracking [`PeerId`]s view of this repo
    ///
    /// Equivalent to `git remote rm`.
    pub fn untrack(&self, peer: &PeerId) -> Result<(), Error> {
        self.storage.untrack(&self.urn, peer).map_err(Error::from)
    }

    /// Retrieve all _directly_ tracked peers
    ///
    /// To retrieve the transitively tracked peers, use [`rad_refs`] and inspect
    /// the `remotes`.
    pub fn tracked(&self) -> Result<Tracked, Error> {
        self.storage.tracked(&self.urn).map_err(Error::from)
    }

    /// Retrieve all directly _as well_ as transitively tracked peers
    pub fn rad_refs(&self) -> Result<Refs, Error> {
        self.storage.rad_refs(&self.urn).map_err(Error::from)
    }

    /// Retrieve the certifier URNs of this repo's identity
    pub fn certifiers(&self) -> Result<HashSet<RadUrn>, Error> {
        self.storage.certifiers(&self.urn).map_err(Error::from)
    }

    /// Check if the given [`git2::Oid`] exists within the context of this repo
    pub fn has_commit(&self, oid: git2::Oid) -> Result<bool, Error> {
        self.storage.has_commit(&self.urn, oid).map_err(Error::from)
    }

    // TODO: find a better way to expose low-level git operations

    pub fn index(&self) -> Result<git2::Index, Error> {
        self.storage.index().map_err(Error::from)
    }

    pub fn find_tree(&self, oid: git2::Oid) -> Result<git2::Tree, Error> {
        self.storage.find_tree(oid).map_err(Error::from)
    }

    pub fn commit(
        &self,
        branch: &str,
        msg: &str,
        tree: &git2::Tree,
        parents: &[&git2::Commit],
    ) -> Result<git2::Oid, Error> {
        self.storage
            .commit(&self.urn, branch, msg, tree, parents)
            .map_err(Error::from)
    }
}

impl TryToOwned for Repo<'_> {
    type Owned = Self;
    type Error = Error;

    fn try_to_owned(&self) -> Result<Self::Owned, Self::Error> {
        let storage = self.storage.try_to_owned().map(TryCow::Owned)?;
        let urn = self.urn.clone();
        Ok(Self { storage, urn })
    }
}