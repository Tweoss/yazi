use std::{collections::BTreeMap, path::Path};

use indexmap::IndexSet;
use serde::{Deserialize, Deserializer};
use yazi_shared::MIME_DIR;

use super::Opener;
use crate::{open::OpenRule, MERGED_YAZI};

#[derive(Debug)]
pub struct Open {
	rules:   Vec<OpenRule>,
	openers: BTreeMap<String, IndexSet<Opener>>,
}

impl Default for Open {
	fn default() -> Self { toml::from_str(&MERGED_YAZI).unwrap() }
}

impl Open {
	pub fn openers<P, M>(&self, path: P, mime: M) -> Option<IndexSet<&Opener>>
	where
		P: AsRef<Path>,
		M: AsRef<str>,
	{
		let is_folder = Some(mime.as_ref() == MIME_DIR);
		self.rules.iter().find_map(|rule| {
			if rule.mime.as_ref().is_some_and(|m| m.matches(&mime))
				|| rule.name.as_ref().is_some_and(|n| n.match_path(&path, is_folder))
			{
				let openers = rule
					.use_
					.iter()
					.filter_map(|use_| self.openers.get(use_))
					.flatten()
					.collect::<IndexSet<_>>();

				if openers.is_empty() { None } else { Some(openers) }
			} else {
				None
			}
		})
	}

	#[inline]
	pub fn block_opener<P, M>(&self, path: P, mime: M) -> Option<&Opener>
	where
		P: AsRef<Path>,
		M: AsRef<str>,
	{
		self.openers(path, mime).and_then(|o| o.into_iter().find(|o| o.block))
	}

	pub fn common_openers(&self, targets: &[(impl AsRef<Path>, impl AsRef<str>)]) -> Vec<&Opener> {
		let grouped: Vec<_> = targets.iter().filter_map(|(p, m)| self.openers(p, m)).collect();
		let flat: IndexSet<_> = grouped.iter().flatten().copied().collect();
		flat.into_iter().filter(|&o| grouped.iter().all(|g| g.contains(o))).collect()
	}
}

impl<'de> Deserialize<'de> for Open {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Outer {
			opener: BTreeMap<String, Vec<Opener>>,
			open:   OuterOpen,
		}
		#[derive(Deserialize)]
		struct OuterOpen {
			rules: Vec<OpenRule>,
		}

		let outer = Outer::deserialize(deserializer)?;
		let openers = outer
			.opener
			.into_iter()
			.map(|(k, v)| (k, v.into_iter().filter_map(|o| o.take()).collect::<IndexSet<_>>()))
			.collect();
		Ok(Self { rules: outer.open.rules, openers })
	}
}
