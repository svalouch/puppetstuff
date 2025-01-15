// SPDX-License-Identifier: GPL-3.0-only

use semver::Version;

/// Specifies how to select the revision that is used by the puppet master
#[derive(Debug, Clone)]
pub enum GitRef {
    /// No specific version, use newest commit in default branch. The commit hash cannot be
    /// determined without contacting the remote repository.
    Head,
    /// A specific commit hash
    Commit(String),
    /// A specific commit pointed to by a tag (exact commit can vary if the tag is moved between
    /// runs).
    Tag(String),
    /// The newest commit in the branch (exact commit depends on the time of check).
    Branch(String),
}

/// Specification where to look for a module in a git repository and how it is handled by g10k.
#[derive(Debug, Clone)]
pub struct GitSpec {
    /// URL to the repository
    pub url: Option<String>,
    /// Git reference, such as a tag or branch name
    pub reference: GitRef,
    /// Fallback-branch if `reference` is a branch
    pub fallback: Option<String>,
    /// If branches should be linked (r10k-specific)
    pub link: bool,
}

/// A module specification from a `Puppetfile`
#[derive(Debug, Clone)]
pub enum Module {
    /// Forge module (name and version)
    Forge(String, Version),
    /// Git repository (name and info)
    Git(String, GitSpec),
}

#[derive(Debug, Clone)]
pub struct BranchMeta {
    pub name: String,
    // pub oid: Oid,
    // pub author_date: Time,
    // pub commit_date: Time,
    // pub author: String,
    pub modules: Vec<Module>,
}
