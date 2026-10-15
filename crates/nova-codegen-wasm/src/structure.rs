//! The shape of a function's control flow, for structuring it (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
//! §4.2): reverse postorder, dominators, merge nodes and loop headers, as
//! Ramsey's "Beyond Relooper" (ICFP 2022) uses them. A graph is reducible
//! when every back edge's target dominates its source; only then can it be
//! nested as `block`s and `loop`s, and otherwise the caller falls back to a
//! dispatch loop.

use nova_mir::{Block, Terminator};

/// No reverse-postorder number: the block is unreachable from the entry.
pub(crate) const UNREACHED: usize = usize::MAX;

/// A function's blocks, analysed.
#[derive(Debug)]
pub(crate) struct Cfg {
    /// The reachable blocks, in reverse postorder from block 0. Only the
    /// tests read it; lowering goes by [`Cfg::rpo_index`].
    #[allow(dead_code)]
    pub rpo: Vec<usize>,
    /// Each block's place in [`Cfg::rpo`], or [`UNREACHED`].
    pub rpo_index: Vec<usize>,
    /// Each block's immediate dominator (block 0's is itself), or
    /// [`UNREACHED`]. Only the tests read it; lowering goes by
    /// [`Cfg::children`].
    #[allow(dead_code)]
    pub idom: Vec<usize>,
    /// Each block's children in the dominator tree, in reverse postorder.
    pub children: Vec<Vec<usize>>,
    /// Whether two or more forward edges enter the block.
    pub merge: Vec<bool>,
    /// Whether a back edge enters the block.
    pub loop_header: Vec<bool>,
    /// Whether every back edge's target dominates its source.
    pub reducible: bool,
}

/// The blocks `term` can go to, once per edge.
pub(crate) fn successors(term: &Terminator) -> Vec<usize> {
    match term {
        Terminator::Goto(b) => vec![b.0 as usize],
        Terminator::Branch { then_, else_, .. } => vec![then_.0 as usize, else_.0 as usize],
        Terminator::Switch { arms, default, .. } => arms
            .iter()
            .map(|(_, b)| b.0 as usize)
            .chain([default.0 as usize])
            .collect(),
        Terminator::Return(_) | Terminator::Trap => Vec::new(),
    }
}

pub(crate) fn analyze(blocks: &[Block]) -> Cfg {
    let n = blocks.len();
    let succs: Vec<Vec<usize>> = blocks.iter().map(|b| successors(&b.term)).collect();

    // Postorder by an explicit stack: a function may have thousands of
    // blocks, and recursion would bound that by this thread's stack.
    let mut visited = vec![false; n];
    let mut post = Vec::with_capacity(n);
    let mut stack: Vec<(usize, usize)> = Vec::new();
    if n > 0 {
        visited[0] = true;
        stack.push((0, 0));
    }
    while let Some(&(b, i)) = stack.last() {
        if let Some(&s) = succs[b].get(i) {
            if let Some(top) = stack.last_mut() {
                top.1 += 1;
            }
            if !visited[s] {
                visited[s] = true;
                stack.push((s, 0));
            }
        } else {
            post.push(b);
            stack.pop();
        }
    }
    let rpo: Vec<usize> = post.into_iter().rev().collect();
    let mut rpo_index = vec![UNREACHED; n];
    for (i, &b) in rpo.iter().enumerate() {
        rpo_index[b] = i;
    }

    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &b in &rpo {
        for &s in &succs[b] {
            preds[s].push(b);
        }
    }

    // Cooper, Harvey and Kennedy, "A Simple, Fast Dominance Algorithm".
    let mut idom = vec![UNREACHED; n];
    if n > 0 {
        idom[0] = 0;
    }
    let intersect = |idom: &[usize], mut a: usize, mut b: usize| {
        while a != b {
            while rpo_index[a] > rpo_index[b] {
                a = idom[a];
            }
            while rpo_index[b] > rpo_index[a] {
                b = idom[b];
            }
        }
        a
    };
    let mut changed = true;
    while changed {
        changed = false;
        for &b in rpo.iter().skip(1) {
            let mut new = UNREACHED;
            for &p in &preds[b] {
                if idom[p] == UNREACHED {
                    continue;
                }
                new = if new == UNREACHED {
                    p
                } else {
                    intersect(&idom, p, new)
                };
            }
            if idom[b] != new {
                idom[b] = new;
                changed = true;
            }
        }
    }

    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &b in rpo.iter().skip(1) {
        children[idom[b]].push(b);
    }
    for c in &mut children {
        c.sort_by_key(|&b| rpo_index[b]);
    }

    let dominates = |a: usize, mut b: usize| loop {
        if a == b {
            return true;
        }
        if b == 0 {
            return false;
        }
        b = idom[b];
    };
    let mut forward_in = vec![0usize; n];
    let mut loop_header = vec![false; n];
    let mut reducible = true;
    for &b in &rpo {
        for &s in &succs[b] {
            if rpo_index[s] <= rpo_index[b] {
                loop_header[s] = true;
                if !dominates(s, b) {
                    reducible = false;
                }
            } else {
                forward_in[s] += 1;
            }
        }
    }
    let merge = forward_in.iter().map(|&k| k >= 2).collect();

    Cfg {
        rpo,
        rpo_index,
        idom,
        children,
        merge,
        loop_header,
        reducible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_mir::{BlockId, Temp};

    fn block(term: Terminator) -> Block {
        Block {
            stmts: Vec::new(),
            term,
        }
    }

    fn goto(b: u32) -> Terminator {
        Terminator::Goto(BlockId(b))
    }

    fn branch(t: u32, e: u32) -> Terminator {
        Terminator::Branch {
            cond: Temp(0),
            then_: BlockId(t),
            else_: BlockId(e),
        }
    }

    /// `if`/`else` joining: 0 branches to 1 and 2, both go to 3.
    #[test]
    fn a_diamond_has_one_merge_node_and_no_loop() {
        let cfg = analyze(&[
            block(branch(1, 2)),
            block(goto(3)),
            block(goto(3)),
            block(Terminator::Return(None)),
        ]);
        assert!(cfg.reducible);
        assert_eq!(cfg.merge, [false, false, false, true]);
        assert_eq!(cfg.loop_header, [false; 4]);
        assert_eq!(cfg.idom, [0, 0, 0, 0]);
        // Depth first from 0 reaches 1, then 3, then 2: reverse postorder is
        // 0, 2, 1, 3.
        assert_eq!(cfg.rpo, [0, 2, 1, 3]);
        assert_eq!(cfg.children[0], [2, 1, 3]);
    }

    /// `while`: 0 → 1 (header) → 2 (body) → 1, and 1 → 3 (exit).
    #[test]
    fn a_while_loop_has_a_header() {
        let cfg = analyze(&[
            block(goto(1)),
            block(branch(2, 3)),
            block(goto(1)),
            block(Terminator::Return(None)),
        ]);
        assert!(cfg.reducible);
        assert_eq!(cfg.loop_header, [false, true, false, false]);
        assert_eq!(cfg.merge, [false; 4]);
        assert_eq!(cfg.idom, [0, 0, 1, 1]);
    }

    /// Two entries into one cycle, 1 ⇄ 2, from 0: no node of the cycle
    /// dominates the other.
    #[test]
    fn a_cycle_with_two_entries_is_irreducible() {
        let cfg = analyze(&[block(branch(1, 2)), block(goto(2)), block(goto(1))]);
        assert!(!cfg.reducible);
    }

    /// A block nothing reaches has no number, and no dominator.
    #[test]
    fn an_unreached_block_is_left_out() {
        let cfg = analyze(&[block(Terminator::Return(None)), block(goto(0))]);
        assert_eq!(cfg.rpo, [0]);
        assert_eq!(cfg.rpo_index[1], UNREACHED);
        assert_eq!(cfg.idom[1], UNREACHED);
    }

    /// The same target twice from one block is two edges, so a merge.
    #[test]
    fn two_edges_to_one_block_make_it_a_merge_node() {
        let cfg = analyze(&[block(branch(1, 1)), block(Terminator::Return(None))]);
        assert_eq!(cfg.merge, [false, true]);
    }
}
