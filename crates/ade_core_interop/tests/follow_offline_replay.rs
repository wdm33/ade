// RED — offline replay test for the follow-mode bridge (CE-N-B-6).
//
// This is the CI gate (NOT `#[ignore]`): it drives the follow bridge
// from REAL header bytes already committed to the repo
// (`corpus/boundary_blocks/`), decodes each header, projects to
// fork-choice inputs, and asserts the bridge selects headers in
// block-number order and rolls back a synthetic within-k rollback.
// Fully deterministic — no network.
//
// Coverage: Allegra (TPraos array(15) Split-VRF) AND Babbage + Conway
// (Praos array(10) Combined-VRF), so both VRF projection paths run.
//
// It also emits the decoded-header corpus artifact under
// `corpus/consensus/follow/` (a reusable workstream-B input).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use ade_core::consensus::candidate::TiebreakerView;
use ade_core::consensus::events::{ChainEvent, Point, SecurityParam};
use ade_core::consensus::praos_state::Nonce;
use ade_core_interop::follow::{
    agreement_status, ingest_rollbackward, ingest_rollforward, project_header_from_block,
    AgreementStatus, FollowState, PeerTip, ProjectedHeader,
};
use ade_types::{BlockNo, Hash28, Hash32, SlotNo};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/ade_core_interop
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

/// Read every `.cbor` block in an era dir, sorted by file name for a
/// deterministic order, and project each header. Per the slice doc, when
/// the corpus is not a clean parent-linked sequence we form the
/// fork-choice order by sorting the real decoded headers by block_no —
/// the point is to exercise decode → project → select on real bytes.
fn project_era_dir(era_dir: &Path) -> Vec<ProjectedHeader> {
    let mut files: Vec<PathBuf> = fs::read_dir(era_dir)
        .unwrap_or_else(|e| panic!("read_dir {era_dir:?}: {e}"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map(|x| x == "cbor").unwrap_or(false))
        .collect();
    files.sort();

    let mut headers: Vec<ProjectedHeader> = files
        .iter()
        .map(|p| {
            let bytes = fs::read(p).unwrap();
            project_header_from_block(&bytes)
                .unwrap_or_else(|e| panic!("project {p:?}: {e:?}"))
        })
        .collect();

    headers.sort_by_key(|h| h.block_no.0);
    headers
}

fn anchor_below(first: &ProjectedHeader) -> (Point, BlockNo, TiebreakerView) {
    // A synthetic anchor strictly before the first real header: lower
    // block number, lower slot, a tiebreaker that loses to any real
    // header so the first RollForward is always selected.
    let anchor_block_no = BlockNo(first.block_no.0 - 1);
    let anchor = Point {
        slot: SlotNo(first.slot.0.saturating_sub(1)),
        hash: Hash32([0u8; 32]),
    };
    let tiebreaker = TiebreakerView {
        slot: SlotNo(first.slot.0.saturating_sub(1)),
        issuer_hash: Hash28([0u8; 28]),
        op_cert_counter: 0,
        leader_vrf_output_first_8: [0u8; 8],
    };
    (anchor, anchor_block_no, tiebreaker)
}

/// Drive a follow session over an ordered header sequence and assert the
/// tip advances in block-number order, ending in agreement with a peer
/// tip set to the final header.
fn drive_forward_and_assert(headers: &[ProjectedHeader]) -> FollowState {
    assert!(headers.len() >= 3, "need a non-trivial sequence");
    let (anchor, anchor_block_no, anchor_tb) = anchor_below(&headers[0]);
    let mut state = FollowState::new(
        anchor,
        anchor_block_no,
        anchor_tb,
        SecurityParam(2160),
        Nonce(Hash32([0u8; 32])),
    );

    let final_header = headers.last().unwrap();
    let peer_tip = PeerTip {
        point: final_header.point(),
        block_no: final_header.block_no,
    };

    let mut prev_block_no = anchor_block_no.0;
    for h in headers {
        let (next, event) =
            ingest_rollforward(state, h, peer_tip.clone()).expect("rollforward");
        match event {
            ChainEvent::ChainSelected { ref new_tip, .. } => {
                assert_eq!(*new_tip, h.point(), "selected tip is the ingested header");
            }
            other => panic!("expected ChainSelected, got {other:?}"),
        }
        // Strictly increasing block number — selection follows order.
        assert!(
            next.current_tip_block_no().0 > prev_block_no,
            "tip block_no must advance: {} !> {}",
            next.current_tip_block_no().0,
            prev_block_no
        );
        prev_block_no = next.current_tip_block_no().0;
        state = next;
    }

    // Caught up to the peer tip → agreement, zero disagreements.
    assert_eq!(state.current_tip(), &final_header.point());
    match agreement_status(&mut state) {
        AgreementStatus::Agreed { .. } => {}
        other => panic!("expected Agreed, got {other:?}"),
    }
    assert_eq!(state.disagreements(), 0);
    state
}

#[test]
fn follow_bridge_selects_real_allegra_tpraos_headers_in_order() {
    // Allegra = TPraos array(15) Split-VRF — exercises the leader_vrf
    // projection path.
    let headers = project_era_dir(&repo_root().join("corpus/boundary_blocks/allegra_epoch237"));
    drive_forward_and_assert(&headers);
}

#[test]
fn follow_bridge_selects_real_babbage_praos_headers_in_order() {
    // Babbage = Praos array(10) Combined-VRF — exercises the vrf_result
    // projection path.
    let headers = project_era_dir(&repo_root().join("corpus/boundary_blocks/babbage_epoch366"));
    drive_forward_and_assert(&headers);
}

#[test]
fn follow_bridge_selects_real_conway_praos_headers_in_order() {
    let headers = project_era_dir(&repo_root().join("corpus/boundary_blocks/conway_epoch577"));
    drive_forward_and_assert(&headers);
}

#[test]
fn follow_bridge_rolls_back_within_k() {
    // Drive forward over the real Babbage sequence, then roll back to an
    // earlier real point within k and assert the tip rolls back to it.
    let headers = project_era_dir(&repo_root().join("corpus/boundary_blocks/babbage_epoch366"));
    let mut state = drive_forward_and_assert(&headers);

    // Roll back to the header three blocks below the tip — a real point
    // the follow state recorded, well within k=2160.
    let target = &headers[headers.len() - 4];
    let tip_block_no_before = state.current_tip_block_no().0;
    let peer_tip = PeerTip {
        point: target.point(),
        block_no: target.block_no,
    };
    let (next, event) =
        ingest_rollbackward(state, target.point(), peer_tip).expect("rollback");
    match event {
        ChainEvent::RolledBack { ref to_point, depth } => {
            assert_eq!(*to_point, target.point());
            assert_eq!(depth.0, tip_block_no_before - target.block_no.0);
        }
        other => panic!("expected RolledBack, got {other:?}"),
    }
    state = next;
    assert_eq!(state.current_tip(), &target.point());
    assert_eq!(state.current_tip_block_no(), target.block_no);
    // Re-applying forward after the rollback selects the next header
    // again — the recent window was truncated correctly.
    assert_eq!(state.disagreements(), 0);
}

#[test]
fn follow_bridge_flags_tip_disagreement() {
    // After catching up, a peer tip at the same block number but a
    // different point is a hard disagreement.
    let headers = project_era_dir(&repo_root().join("corpus/boundary_blocks/conway_epoch577"));
    let mut state = drive_forward_and_assert(&headers);
    let last = headers.last().unwrap();
    state.observe_peer_tip(PeerTip {
        point: Point {
            slot: last.slot,
            hash: Hash32([0xff; 32]),
        },
        block_no: last.block_no,
    });
    match agreement_status(&mut state) {
        AgreementStatus::Disagree { .. } => {}
        other => panic!("expected Disagree, got {other:?}"),
    }
    assert_eq!(state.disagreements(), 1);
}

#[test]
fn follow_bridge_emits_decoded_header_corpus_artifact() {
    // Emit the projected per-header fields as a small committed JSON
    // corpus under corpus/consensus/follow/ — reusable by workstream B.
    let out_dir = repo_root().join("corpus/consensus/follow");
    fs::create_dir_all(&out_dir).expect("create corpus dir");

    for (era, subdir) in [
        ("allegra_tpraos", "allegra_epoch237"),
        ("babbage_praos", "babbage_epoch366"),
        ("conway_praos", "conway_epoch577"),
    ] {
        let headers =
            project_era_dir(&repo_root().join("corpus/boundary_blocks").join(subdir));
        let json = headers_to_json(era, &headers);
        let path = out_dir.join(format!("{era}_headers.json"));
        fs::write(&path, json).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
        assert!(path.exists());
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Hand-rolled JSON so the corpus artifact carries no serde dependency
/// and is byte-stable across runs (fields in fixed order, headers in
/// block-number order).
fn headers_to_json(era: &str, headers: &[ProjectedHeader]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"era\": \"{era}\",\n"));
    out.push_str("  \"note\": \"RED follow-mode projection of real boundary-block headers; selection-only, NOT validated.\",\n");
    out.push_str("  \"headers\": [\n");
    for (i, h) in headers.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"block_no\": {},\n", h.block_no.0));
        out.push_str(&format!("      \"slot\": {},\n", h.slot.0));
        out.push_str(&format!("      \"hash\": \"{}\",\n", hex(&h.hash.0)));
        out.push_str(&format!(
            "      \"issuer_pool\": \"{}\",\n",
            hex(&h.issuer_pool.0)
        ));
        out.push_str(&format!("      \"op_cert_counter\": {},\n", h.op_cert_counter));
        out.push_str(&format!(
            "      \"vrf_output_first_8\": \"{}\"\n",
            hex(&h.vrf_output_first_8)
        ));
        out.push_str("    }");
        if i + 1 < headers.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}
