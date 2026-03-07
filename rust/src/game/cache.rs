// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements all functionality to search for eligible caches and select one according
//! to the chose policy.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use serde::Deserialize;

use crate::constraint::{ConstraintGetters, ConstraintImpact};
use crate::game::Game;

/// generic way of specifying something which can be used as cache
pub trait CachableSpec {
    /// create a new cache-specification
    fn new(event_name: String, path: PathBuf) -> Self;
    /// the path which backs this cache
    fn path(&self) -> &PathBuf;
    /// the name of the event this cache is associated with
    fn event_name(&self) -> &str;
    /// determines whether the cache exists (is available to choose)
    fn exists(&self) -> bool;
}

/// specifies one cache
#[derive(Clone)]
pub struct CacheSpec {
    /// the event this cache is associated with
    event_name: String,
    /// a path to the cache stored on disk
    path: PathBuf,
}

impl CachableSpec for CacheSpec {
    fn new(event_name: String, path: PathBuf) -> Self {
        Self { event_name, path }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn event_name(&self) -> &str {
        &self.event_name
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Compute cache file candidates (path + label) for the current `GameParse`.
///
/// This function only *theoretically* computes what valid identifiers for caches would be. It does
/// not check if these files exist.
#[must_use]
pub(super) fn get_caches<T, S>(initial_hash: u64, constraints: &[T]) -> Vec<S>
where
    T: Hash + ConstraintGetters + ConstraintImpact,
    S: CachableSpec,
{
    let cache_dir = Path::new("./.cache/");

    // collect hashes for each "layer" of constraints
    let mut input_hashes = vec![];
    let mut prev_hash = initial_hash;
    for c in constraints.iter() {
        // hash c as a new layer to the previous hash
        let mut hasher = DefaultHasher::new();
        prev_hash.hash(&mut hasher);
        if c.has_impact() {
            c.hash(&mut hasher);
            prev_hash = hasher.finish();
            input_hashes.push(S::new(
                c.type_str(),
                cache_dir
                    .join(format!("{:x}", prev_hash))
                    .with_extension("cache"),
            ));
        }
    }
    input_hashes
}

/// argument specification for `CacheMode` so this can be used with clap
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CacheModeArg {
    /// select the most recent available cache
    MostRecent,
    /// select no cache
    None,
    /// select a cache based on its path
    SpecificCache,
    /// select a cache based on the event-name
    SpecificEvent,
}

/// specifies the strategy to choose a cache
#[derive(Deserialize, Debug, Clone)]
pub enum CacheMode {
    /// select the most recent available cache
    MostRecent,
    /// select no cache
    None,
    /// select a cache based on its path
    SpecificCache(PathBuf),
    /// select a cache based on the event-name
    SpecificEvent(String),
}

impl CacheModeArg {
    /// convert the cachemode parsed to a full `CacheMode`
    pub fn finalize(&self, cache: &Option<PathBuf>, event: &Option<String>) -> Result<CacheMode> {
        match &self {
            CacheModeArg::MostRecent => Ok(CacheMode::MostRecent),
            CacheModeArg::None => Ok(CacheMode::None),
            CacheModeArg::SpecificCache => {
                let Some(cache) = cache else {
                    bail!("Didn't specify which cache to use")
                };
                Ok(CacheMode::SpecificCache(cache.to_path_buf()))
            }
            CacheModeArg::SpecificEvent => {
                let Some(event) = event else {
                    bail!("Didn't specify which event to use")
                };
                Ok(CacheMode::SpecificEvent(event.to_string()))
            }
        }
    }
}

/// specifies the strategy used as fallback if the cache could not be found
#[derive(Debug, Clone, clap::ValueEnum, Deserialize)]
pub enum CacheModeFallback {
    /// select the most recent available cache
    MostRecent,
    /// no cache should be selected
    None,
}

impl CacheModeFallback {
    /// select a cache as fallback according to the specification
    ///
    /// The selectable caches need to be gathered in advance and provided as `caches`.
    #[must_use]
    fn select_cache<'a, S: CachableSpec>(&self, caches: &'a [S]) -> Option<&'a S> {
        match &self {
            CacheModeFallback::MostRecent => {
                caches
                    .iter()
                    // want to use the most recent one
                    .rev()
                    // using the last one doesn't make sense, nothing would be left to compute
                    .skip(1)
                    // only consider existing caches
                    .find(|c| c.exists())
            }
            CacheModeFallback::None => None,
        }
    }
}

impl CacheMode {
    /// select a cache according to the specification if the this doesn't work a fallback can be
    /// provided.
    ///
    /// The selectable caches need to be gathered in advance and provided as `caches`.
    pub(crate) fn select_cache<'a, S: CachableSpec>(
        &self,
        fallback: &Option<CacheModeFallback>,
        caches: &'a [S],
    ) -> Option<&'a S> {
        match &self {
            CacheMode::MostRecent => {
                // re-use same logic like in fallback
                CacheModeFallback::MostRecent.select_cache(caches)
            }
            CacheMode::SpecificCache(path_buf) => caches
                .iter()
                .find(|c| c.path() == path_buf)
                .filter(|c| c.exists())
                .or_else(|| fallback.as_ref().and_then(|f| f.select_cache(caches))),
            CacheMode::SpecificEvent(name) => caches
                .iter()
                .find(|c| c.event_name() == name)
                .filter(|c| c.exists())
                .or_else(|| fallback.as_ref().and_then(|f| f.select_cache(caches))),
            CacheMode::None => {
                // re-use same logic like in fallback
                CacheModeFallback::None.select_cache(caches)
            }
        }
    }
}

impl Game {
    /// obtain the cache-candidates for this game
    #[must_use]
    pub fn get_cache_candidates<S: CachableSpec>(&mut self) -> Vec<S> {
        let initial_hash = {
            let mut hasher = DefaultHasher::new();
            self.map_a.hash(&mut hasher);
            self.map_b.hash(&mut hasher);
            hasher.finish()
        };
        get_caches(initial_hash, &self.constraints_orig)
    }

    /// select a cache according to the specified strategy/strategies
    ///
    /// Needs to be provided `caches`, the list of cache-candidates
    ///
    /// With `output` it can be decided whether this should print whether and which cache shall be
    /// used
    pub fn select_cache<S: CachableSpec + Clone>(
        &mut self,
        caches: &[S],
        mode: CacheMode,
        fallback: &Option<CacheModeFallback>,
        output: bool,
    ) -> Result<()> {
        let selected = mode
            .select_cache(fallback, caches)
            .context("no cache found")?;

        self.cache_file = Some(selected.path().to_path_buf());
        if output {
            println!("Selected cache {:?}", self.cache_file);
        }
        Ok(())
    }

    /// set whether a cache should be generated in the end
    ///
    /// With `output` it can be decided whether this should print whether and which cache shall be
    /// used
    pub fn set_gen_cache<S: CachableSpec>(&mut self, caches: &[S], output: bool) -> Result<()> {
        self.cache_to = caches.last().map(|x| x.path().clone());
        if output {
            println!("Write cache to {:?}", self.cache_to);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rust_decimal::Decimal;

    use crate::matching_repr::MaskedMatching;

    use super::*;

    #[derive(Debug, Default)]
    struct MockConstraint {
        typ: String,
        impact: bool,
        comment: String,
        num: Decimal,
        map: MaskedMatching,
    }
    impl ConstraintImpact for MockConstraint {
        fn has_impact(&self) -> bool {
            self.impact
        }
    }
    impl ConstraintGetters for MockConstraint {
        fn type_str(&self) -> String {
            self.typ.clone()
        }
        fn num(&self) -> Decimal {
            self.num
        }
        fn comment(&self) -> &str {
            &self.comment
        }
        fn matching(&self) -> &MaskedMatching {
            &self.map
        }
    }
    impl Hash for MockConstraint {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.typ.hash(state);
            self.impact.hash(state);
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Default)]
    struct MockSpec {
        event_name: String,
        path: PathBuf,
        exists: bool,
    }
    impl CachableSpec for MockSpec {
        fn new(event_name: String, path: PathBuf) -> Self {
            Self {
                event_name,
                path,
                exists: false,
            }
        }
        fn path(&self) -> &PathBuf {
            &self.path
        }
        fn event_name(&self) -> &str {
            &self.event_name
        }
        fn exists(&self) -> bool {
            self.exists
        }
    }

    #[test]
    fn get_caches_respects_impact_flag() {
        let init_hash = 0xdeadbeefu64;
        let constraints = [
            MockConstraint {
                typ: "A".to_string(),
                impact: true,
                ..Default::default()
            },
            MockConstraint {
                typ: "B".to_string(),
                impact: false,
                ..Default::default()
            }, // should be skipped
            MockConstraint {
                typ: "C".to_string(),
                impact: true,
                ..Default::default()
            },
        ];

        let caches: Vec<MockSpec> = get_caches(init_hash, &constraints);
        assert_eq!(caches.len(), 2);
        assert_eq!(caches[0].event_name(), "A");
        assert_eq!(caches[1].event_name(), "C");
        // paths stay unchecked as I did not re-compute the hash chain manually
    }

    #[test]
    fn finalize_requires_missing_args() {
        let arg = CacheModeArg::SpecificCache;
        assert!(arg.finalize(&None, &None).is_err());

        let arg = CacheModeArg::SpecificEvent;
        assert!(arg.finalize(&None, &Some("my_event".into())).is_ok());
    }

    #[test]
    fn finalize_not_requires_args() {
        let arg = CacheModeArg::None;
        assert!(arg.finalize(&None, &None).is_ok());

        let arg = CacheModeArg::MostRecent;
        assert!(arg.finalize(&None, &None).is_ok());
    }

    #[test]
    fn fallback_select_cache_most_recent() {
        // now is skipped always
        // new is skipped because it does not exist
        // => should select mid
        let specs = vec![
            MockSpec {
                event_name: "old".into(),
                path: PathBuf::from("old.cache"),
                exists: true,
            },
            MockSpec {
                event_name: "mid".into(),
                path: PathBuf::from("mid.cache"),
                exists: true,
            },
            MockSpec {
                event_name: "new".into(),
                path: PathBuf::from("new.cache"),
                exists: false,
            },
            MockSpec {
                event_name: "now".into(),
                path: PathBuf::from("now.cache"),
                exists: true,
            },
        ];
        let chosen = CacheModeFallback::MostRecent.select_cache(&specs);
        assert_eq!(chosen.unwrap().event_name(), "mid");
    }

    #[test]
    fn mode_select_cache_falls_back_when_missing() {
        let specs = vec![
            MockSpec {
                event_name: "alpha".into(),
                path: PathBuf::from("a.cache"),
                exists: false,
            },
            MockSpec {
                event_name: "beta".into(),
                path: PathBuf::from("b.cache"),
                exists: true,
            },
            MockSpec {
                event_name: "now".into(),
                path: PathBuf::from("now.cache"),
                exists: true,
            },
        ];

        let mode = CacheMode::SpecificEvent("gamma".into()); // not present

        let fallback = Some(CacheModeFallback::MostRecent);

        let chosen = mode.select_cache(&fallback, &specs);
        // Should fall back to the most recent existing cache (excluding now) "beta"
        assert_eq!(chosen.unwrap().event_name(), "beta");
    }

    #[test]
    fn mode_select_cache_none_when_no_fallback() {
        let specs = vec![
            MockSpec {
                event_name: "alpha".into(),
                path: PathBuf::from("a.cache"),
                exists: false,
            },
            MockSpec {
                event_name: "beta".into(),
                path: PathBuf::from("b.cache"),
                exists: true,
            },
            MockSpec {
                event_name: "now".into(),
                path: PathBuf::from("now.cache"),
                exists: true,
            },
        ];

        let mode = CacheMode::SpecificEvent("gamma".into()); // not present

        let chosen = mode.select_cache(&None, &specs);
        assert!(chosen.is_none());
    }
}
