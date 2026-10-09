//! The resolver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4): one version of each package name per build, caret ranges, and a
//! backtracking search that keeps locked versions.

use std::collections::{BTreeMap, HashMap};

use nova_diagnostics::{Diagnostic, Span};
use semver::{Version, VersionReq};

use crate::lock::{Lock, LockedPackage};

/// One version of a package, as the index lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub version: Version,
    /// Its `[dependencies]`: names and requirements.
    pub deps: Vec<(String, VersionReq)>,
    /// The SHA-256 of its tarball.
    pub checksum: String,
}

/// What the resolver asks of an index.
pub trait IndexView {
    /// Every version of `name`, or `Ok(None)` when the index has no
    /// package of that name. `Err` when the index cannot be read.
    fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String>;
}

/// A requirement on a package, and where it came from.
#[derive(Debug, Clone)]
pub struct Requirement {
    pub name: String,
    pub req: VersionReq,
    /// The package that made it, named in M0015.
    pub by: String,
    /// The root's manifest entry through which it is reached, where M0014
    /// and M0015 point (spec §4.5).
    pub span: Span,
}

/// What a resolution may change (spec §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unlock {
    /// Keep every locked version that still fits. When they cannot all
    /// stay, resolve again as if there were no lock.
    Nothing,
    /// `nova update`: ignore the lock.
    All,
    /// `nova update <name>`: only that name may move.
    One(String),
}

/// Why a resolution failed.
#[derive(Debug)]
pub enum ResolveError {
    /// M0014 or M0015, ready to render.
    Diagnostic(Diagnostic),
    /// The index could not be read: why.
    Index(String),
}

/// The packages `requirements` need, newest versions first, keeping
/// `lock`'s versions as `unlock` says. Sorted by name, each with its
/// dependencies' names.
pub fn resolve(
    requirements: &[Requirement],
    lock: Option<&Lock>,
    unlock: &Unlock,
    index: &mut dyn IndexView,
) -> Result<Vec<LockedPackage>, ResolveError> {
    let pins = |except: Option<&str>| -> HashMap<String, Version> {
        lock.map(|lock| {
            lock.packages
                .iter()
                .filter(|p| Some(p.name.as_str()) != except)
                .map(|p| (p.name.clone(), p.version.clone()))
                .collect()
        })
        .unwrap_or_default()
    };
    let mut search = Search {
        index,
        known: HashMap::new(),
        pinned: HashMap::new(),
        strict: false,
        failure: None,
        conflicted: Vec::new(),
    };
    match unlock {
        Unlock::All => search.run(requirements, HashMap::new(), false),
        Unlock::One(name) => search
            .run(requirements, pins(Some(name.as_str())), true)
            .map_err(|error| match error {
                ResolveError::Diagnostic(d) if d.code == "M0015" => ResolveError::Diagnostic(
                    d.with_note("the other locked versions were kept; run `nova update` to let them move too"),
                ),
                other => other,
            }),
        Unlock::Nothing => {
            // Keep every locked version that can stay (spec §4.3): when the
            // pinned search meets a conflict, unpin only the packages it
            // names and search again, until it succeeds or nothing is left
            // pinned.
            let mut pinned = pins(None);
            loop {
                match search.run(requirements, pinned.clone(), false) {
                    Err(ResolveError::Diagnostic(d)) if d.code == "M0015" && !pinned.is_empty() => {
                        let before = pinned.len();
                        for name in &search.conflicted {
                            pinned.remove(name);
                        }
                        // A conflict naming no pinned package: unpin them all.
                        if pinned.len() == before {
                            pinned.clear();
                        }
                    }
                    result => return result,
                }
            }
        }
    }
}

/// Why a search failed: a package, and every requirement on it.
struct Failure {
    name: String,
    reqs: Vec<Requirement>,
}

struct State {
    reqs: Vec<Requirement>,
    /// Package names in the order they were first reached.
    order: Vec<String>,
    chosen: BTreeMap<String, Candidate>,
}

struct Search<'a> {
    index: &'a mut dyn IndexView,
    /// Every version of each package asked for so far, newest first.
    known: HashMap<String, Vec<Candidate>>,
    /// Locked versions to keep.
    pinned: HashMap<String, Version>,
    /// Whether a pinned version that does not fit leaves no candidate
    /// (`nova update <name>`) rather than letting its package move.
    strict: bool,
    /// The first conflict met, for M0015.
    failure: Option<Failure>,
    /// After a failed search: the package of its conflict, and every
    /// package that made a requirement on it.
    conflicted: Vec<String>,
}

impl Search<'_> {
    fn run(
        &mut self,
        requirements: &[Requirement],
        pinned: HashMap<String, Version>,
        strict: bool,
    ) -> Result<Vec<LockedPackage>, ResolveError> {
        self.pinned = pinned;
        self.strict = strict;
        self.failure = None;
        let mut order: Vec<String> = requirements.iter().map(|r| r.name.clone()).collect();
        order.sort();
        order.dedup();
        let mut state = State {
            reqs: requirements.to_vec(),
            order,
            chosen: BTreeMap::new(),
        };
        if self.solve(&mut state)? {
            return Ok(state
                .chosen
                .into_iter()
                .map(|(name, candidate)| {
                    let mut dependencies: Vec<String> =
                        candidate.deps.iter().map(|(n, _)| n.clone()).collect();
                    dependencies.sort();
                    dependencies.dedup();
                    LockedPackage {
                        name,
                        version: candidate.version,
                        checksum: candidate.checksum,
                        dependencies,
                    }
                })
                .collect());
        }
        let failure = self.failure.take().expect("a failed search records why");
        self.conflicted = std::iter::once(failure.name.clone())
            .chain(failure.reqs.iter().map(|r| r.by.clone()))
            .collect();
        Err(ResolveError::Diagnostic(conflict(&failure)))
    }

    /// Choose a version for the next unchosen package, depth first.
    fn solve(&mut self, state: &mut State) -> Result<bool, ResolveError> {
        let Some(name) = state
            .order
            .iter()
            .find(|n| !state.chosen.contains_key(*n))
            .cloned()
        else {
            return Ok(true);
        };
        let reqs: Vec<Requirement> = state
            .reqs
            .iter()
            .filter(|r| r.name == name)
            .cloned()
            .collect();
        let fitting: Vec<Candidate> = self
            .versions(&name, &reqs)?
            .into_iter()
            .filter(|c| reqs.iter().all(|r| r.req.matches(&c.version)))
            .collect();
        let kept = self
            .pinned
            .get(&name)
            .map(|version| fitting.iter().position(|c| &c.version == version));
        let tried = match kept {
            // A locked version that still fits is the only candidate.
            Some(Some(i)) => vec![fitting[i].clone()],
            Some(None) if self.strict => Vec::new(),
            _ => fitting,
        };
        let via = reqs[0].span;
        for candidate in tried {
            let reqs_before = state.reqs.len();
            let order_before = state.order.len();
            let mut new_names = Vec::new();
            for (dep, req) in &candidate.deps {
                state.reqs.push(Requirement {
                    name: dep.clone(),
                    req: req.clone(),
                    by: name.clone(),
                    span: via,
                });
                if !state.order.contains(dep) && !new_names.contains(dep) {
                    new_names.push(dep.clone());
                }
            }
            new_names.sort();
            state.order.extend(new_names);
            // A new requirement on a package already chosen must fit it.
            let clash = candidate.deps.iter().find(|(dep, req)| {
                state
                    .chosen
                    .get(dep)
                    .is_some_and(|chosen| !req.matches(&chosen.version))
            });
            match clash {
                Some((dep, _)) => {
                    let on_dep = state
                        .reqs
                        .iter()
                        .filter(|r| &r.name == dep)
                        .cloned()
                        .collect();
                    self.note(dep, on_dep);
                }
                None => {
                    state.chosen.insert(name.clone(), candidate.clone());
                    if self.solve(state)? {
                        return Ok(true);
                    }
                    state.chosen.remove(&name);
                }
            }
            state.order.truncate(order_before);
            state.reqs.truncate(reqs_before);
        }
        self.note(&name, reqs);
        Ok(false)
    }

    /// Every version of `name`, newest first; M0014 when there is none.
    fn versions(
        &mut self,
        name: &str,
        reqs: &[Requirement],
    ) -> Result<Vec<Candidate>, ResolveError> {
        if !self.known.contains_key(name) {
            let Some(mut versions) = self.index.versions(name).map_err(ResolveError::Index)? else {
                return Err(ResolveError::Diagnostic(
                    Diagnostic::error("M0014", format!("the index has no package `{name}`"))
                        .with_primary_label(reqs[0].span, "required here"),
                ));
            };
            versions.sort_by(|a, b| b.version.cmp(&a.version));
            self.known.insert(name.to_string(), versions);
        }
        Ok(self.known[name].clone())
    }

    /// Remember the first conflict met.
    fn note(&mut self, name: &str, reqs: Vec<Requirement>) {
        if self.failure.is_none() {
            self.failure = Some(Failure {
                name: name.to_string(),
                reqs,
            });
        }
    }
}

/// M0015: no version of a package meets every requirement on it.
fn conflict(failure: &Failure) -> Diagnostic {
    let wants: Vec<String> = failure
        .reqs
        .iter()
        .map(|r| format!("`{}` from `{}`", r.req, r.by))
        .collect();
    Diagnostic::error(
        "M0015",
        format!(
            "no version of `{}` meets every requirement: {}",
            failure.name,
            wants.join(", ")
        ),
    )
    .with_primary_label(failure.reqs[0].span, "required here")
}
