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
// Fuzzy engine
#[derive(Default)]
pub struct FuzzyEngineBuilder {
	query: String,
	case: CaseMatching,
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

	pub fn rank_builder(
		mut self,
		rank_builder: Arc<RankBuilder>,
	) -> Self {
		self.rank_builder = rank_builder;
		self
	}

	#[allow(deprecated)]
	pub fn build(self) -> FuzzyEngine {
		let matcher: RwLock<Matcher> = RwLock::new({
			let mut matcher = Matcher::default();
			if self.case == CaseMatching::Respect {
				matcher.config.ignore_case = false;
			}
			matcher
		});

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

		let mut n_buf = Vec::new();
		let needle = nucleo::Utf32Str::new(pattern, &mut n_buf);
		let mut h_buf = Vec::new();
		let haystack = nucleo::Utf32Str::new(choice, &mut h_buf);

		if let Ok(mut matcher) = self.matcher.write() {
			let mut indices: Vec<u32> = Vec::with_capacity(haystack.len());
			if let Some(i) = matcher.fuzzy_indices(haystack, needle, &mut indices) {
				let res: Result<Vec<usize>, _> =
					indices.into_iter().map(TryFrom::try_from).collect();
				res.ok().map(|r| (i as i64, r))
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
