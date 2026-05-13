//! Reciprocal Rank Fusion (RRF) for multi-stream recall.
//!
//! ARCHITECTURE §6.1 formula: `score(d) = Σ 1/(k + rank_stream(d))`
//! across every stream the doc appears in (default `k=60`). Ranks are
//! 1-indexed because the formula is sensitive to position 0 (1/(k+0) ==
//! 1/k makes the top hit infinite-ish).
//!
//! The function accepts pre-ranked streams as `&[(stream, hits)]` and
//! returns a fused `Vec<Hit>` truncated to `top_k`. Per-doc
//! `streams_matched` is the union of contributing streams; per-doc
//! `score` is the RRF sum (NOT a similarity — it is a fused rank score).

use std::collections::BTreeMap;

use crate::hit::{Hit, Metadata};
use crate::stream::Stream;

/// Default RRF constant from Cormack et al. 2009. Smaller `k` weights
/// the head more aggressively; `k=60` is the well-tested sweet spot.
pub const RRF_K: f64 = 60.0;

/// Fuse per-stream ranked hit lists into a single ranked list.
///
/// `streams` is a slice of `(Stream, Vec<Hit>)` pairs. Each `Vec<Hit>`
/// MUST already be sorted best-first by the source stream — RRF only
/// looks at array index. Ties on RRF score break by `doc_id` ASC.
#[must_use]
pub fn rrf_fuse(streams: &[(Stream, Vec<Hit>)], top_k: usize) -> Vec<Hit> {
    let mut acc: BTreeMap<String, FusedDoc> = BTreeMap::new();
    for (stream, hits) in streams {
        accumulate_stream(*stream, hits, &mut acc);
    }
    let mut fused: Vec<Hit> = acc.into_values().map(FusedDoc::into_hit).collect();
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.doc_id.cmp(&b.doc_id))
    });
    fused.truncate(top_k);
    fused
}

struct FusedDoc {
    doc_id: String,
    score: f64,
    snippet: String,
    source_path: String,
    streams_matched: Vec<Stream>,
    metadata: Metadata,
}

impl FusedDoc {
    fn into_hit(self) -> Hit {
        Hit {
            doc_id: self.doc_id,
            score: self.score,
            snippet: self.snippet,
            source_path: self.source_path,
            streams_matched: self.streams_matched,
            metadata: self.metadata,
        }
    }
}

fn accumulate_stream(stream: Stream, hits: &[Hit], acc: &mut BTreeMap<String, FusedDoc>) {
    for (rank0, hit) in hits.iter().enumerate() {
        let rank1 = rank0 + 1;
        #[expect(
            clippy::cast_precision_loss,
            reason = "rank1 is bounded by top-k <= a few hundred; f64 covers it exactly."
        )]
        let contribution = 1.0_f64 / (RRF_K + rank1 as f64);
        let entry = acc.entry(hit.doc_id.clone()).or_insert_with(|| FusedDoc {
            doc_id: hit.doc_id.clone(),
            score: 0.0,
            snippet: hit.snippet.clone(),
            source_path: hit.source_path.clone(),
            streams_matched: Vec::new(),
            metadata: hit.metadata.clone(),
        });
        entry.score += contribution;
        if !entry.streams_matched.contains(&stream) {
            entry.streams_matched.push(stream);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stream, rrf_fuse};
    use crate::hit::Hit;

    fn h(id: &str) -> Hit {
        Hit::single_stream(id.into(), 0.0, String::new(), String::new(), Stream::Bm25)
    }

    #[test]
    fn single_stream_round_trips_top_k() {
        let bm25 = vec![h("a"), h("b"), h("c")];
        let fused = rrf_fuse(&[(Stream::Bm25, bm25)], 2);
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].doc_id, "a");
        assert_eq!(fused[1].doc_id, "b");
    }

    #[test]
    fn cross_stream_doc_accumulates_both_streams() {
        let bm25 = vec![h("a"), h("b")];
        let vector = vec![h("b"), h("c")];
        let fused = rrf_fuse(&[(Stream::Bm25, bm25), (Stream::Vector, vector)], 3);
        let b = fused
            .iter()
            .find(|x| x.doc_id == "b")
            .expect("b survives fusion");
        assert!(b.streams_matched.contains(&Stream::Bm25));
        assert!(b.streams_matched.contains(&Stream::Vector));
        assert!(b.score > fused.iter().find(|x| x.doc_id == "c").unwrap().score);
    }

    #[test]
    fn empty_streams_yield_empty_result() {
        let out = rrf_fuse(&[], 5);
        assert!(out.is_empty());
    }
}
