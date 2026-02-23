/// This module implements all functionality to search for eligible caches and select one according
/// to the chose policy.
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use serde::Deserialize;

use crate::constraint::{ConstraintGetters, ConstraintImpact};
use crate::game::Game;

pub trait CachableSpec {
    fn new(event_name: String, path: PathBuf) -> Self;
    fn path(&self) -> &PathBuf;
    fn event_name(&self) -> &str;
    fn exists(&self) -> bool;
}

#[derive(Clone)]
pub struct CacheSpec {
    event_name: String,
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
/// This function **does not** access the filesystem: it deterministically
/// computes a set of hashed cache file paths (useful for `select_cache` and tests).
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

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CacheModeArg {
    MostRecent,
    None,
    SpecificCache,
    SpecificEvent,
}

#[derive(Deserialize, Debug, Clone)]
pub enum CacheMode {
    MostRecent,
    None,
    SpecificCache(PathBuf),
    SpecificEvent(String),
}

impl CacheModeArg {
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

#[derive(Debug, Clone, clap::ValueEnum, Deserialize)]
pub enum CacheModeFallback {
    MostRecent,
    None,
}

impl CacheModeFallback {
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
    pub fn select_cache<'a, S: CachableSpec>(
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

    use super::*;

    #[derive(Debug, Default)]
    struct MockConstraint {
        typ: String,
        impact: bool,
        comment: String,
        num: Decimal,
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
