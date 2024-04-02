use std::fmt::{Display, Error, Formatter};
use std::sync::Arc;

use nucleo::Matcher;

use crate::item::RankBuilder;
use crate::{CaseMatching, MatchEngine};
use crate::{MatchRange, MatchResult, SkimItem};
use bitflags::_core::cmp::min;
use std::convert::TryFrom;
use std::sync::RwLock;

//------------------------------------------------------------------------------
#[derive(Debug, Copy, Clone)]
#[derive(Default)]
pub enum FuzzyAlgorithm {
	SkimV1,
	#[default]
 SkimV2,
	Clangd,
}

impl FuzzyAlgorithm {
	pub fn of(algorithm: &str) -> Self {
		match algorithm.to_ascii_lowercase().as_ref() {
			"skim_v1" => FuzzyAlgorithm::SkimV1,
			"skim_v2" | "skim" => FuzzyAlgorithm::SkimV2,
			"clangd" => FuzzyAlgorithm::Clangd,
			_ => FuzzyAlgorithm::SkimV2,
		}
	}
}



//------------------------------------------------------------------------------
// Fuzzy engine
#[derive(Default)]
pub struct FuzzyEngineBuilder {
	query: String,
	case: CaseMatching,
	algorithm: FuzzyAlgorithm,
	rank_builder: Arc<RankBuilder>,
}

impl FuzzyEngineBuilder {
	pub fn query(
		mut self,
		query: &str,
	) -> Self {
		self.query = query.to_string();
		self
	}

	pub fn case(
		mut self,
		case: CaseMatching,
	) -> Self {
		self.case = case;
		self
	}

	pub fn algorithm(
		mut self,
		algorithm: FuzzyAlgorithm,
	) -> Self {
		self.algorithm = algorithm;
		self
	}

	pub fn rank_builder(
		mut self,
		rank_builder: Arc<RankBuilder>,
	) -> Self {
		self.rank_builder = rank_builder;
		self
	}

	#[allow(deprecated)]
	pub fn build(self) -> FuzzyEngine {
		let matcher: RwLock<Matcher> = RwLock::new(Matcher::default());

		FuzzyEngine {
			matcher,
			query: self.query,
			rank_builder: self.rank_builder,
		}
	}
}

pub struct FuzzyEngine {
	query: String,
	matcher: RwLock<Matcher>,
	rank_builder: Arc<RankBuilder>,
}

impl FuzzyEngine {
	pub fn builder() -> FuzzyEngineBuilder {
		FuzzyEngineBuilder::default()
	}

	fn fuzzy_match(
		&self,
		choice: &str,
		pattern: &str,
	) -> Option<(i64, Vec<usize>)> {
		if pattern.is_empty() {
			return Some((0, Vec::new()));
		} else if choice.is_empty() {
			return None;
		}

		let mut buf = Vec::new();
		let needle = nucleo::Utf32Str::new(pattern, &mut buf);
		let mut buf = Vec::new();
		let haystack = nucleo::Utf32Str::new(choice, &mut buf);

		let mut indices: Vec<u32> = vec![];
		let matcher = self.matcher.write().ok();
		if let Some(mut m) = matcher {
			if let Some(u) = m.fuzzy_indices(haystack, needle, &mut indices) {
				let res: Result<Vec<usize>, _> =
					indices.into_iter().map(TryFrom::try_from).collect();
				res.ok().map(|r| (u as i64, r))
			} else {
				None
			}
		} else {
			None
		}
	}
}

impl MatchEngine for FuzzyEngine {
	fn match_item(
		&self,
		item: Arc<dyn SkimItem>,
	) -> Option<MatchResult> {
		// iterate over all matching fields:
		let mut matched_result = None;
		let item_text = item.text();
		let default_range = [(0, item_text.len())];
		for &(start, end) in item.get_matching_ranges().unwrap_or(&default_range) {
			let start = min(start, item_text.len());
			let end = min(end, item_text.len());
			matched_result =
				self.fuzzy_match(&item_text[start..end], &self.query)
					.map(|(s, vec)| {
						if start != 0 {
							let start_char = &item_text[..start].chars().count();
							(s, vec.iter().map(|x| x + start_char).collect())
						} else {
							(s, vec)
						}
					});

			if matched_result.is_some() {
				break;
			}
		}

		matched_result.as_ref()?;

		let (score, matched_range) = matched_result.unwrap();

		let begin = *matched_range.first().unwrap_or(&0);
		let end = *matched_range.last().unwrap_or(&0);

		let item_len = item_text.len();
		Some(MatchResult {
			rank: self
				.rank_builder
				.build_rank(score as i32, begin, end, item_len),
			matched_range: MatchRange::Chars(matched_range),
		})
	}
}

impl Display for FuzzyEngine {
	fn fmt(
		&self,
		f: &mut Formatter,
	) -> Result<(), Error> {
		write!(f, "(Fuzzy: {})", self.query)
	}
}
