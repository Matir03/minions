use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use crate::ai::captain::board_node::BoardNodeRef;
use crate::ai::explore::SearchTree;
use crate::ai::game_node::GameNodeRef;
use crate::ai::general_node::GeneralNodeRef;
use crate::heuristics::Heuristic;

struct GraphvizBuilder {
    // clusters
    game_cluster: String,
    general_cluster: String,
    board_clusters: Vec<String>,
    cross_edges: String,

    // visited sets
    seen_game: HashSet<usize>,
    seen_general: HashSet<usize>,
    seen_board: HashSet<usize>,

    // per-rank groupings for LR layout
    // Game: key by absolute ply from game state (sorted by BTreeMap)
    game_ranks: BTreeMap<usize, Vec<String>>,
    // General: depth from the root general node
    general_ranks: Vec<Vec<String>>,
    // Boards: [board_idx][depth]
    board_ranks: Vec<Vec<Vec<String>>>,
}

impl GraphvizBuilder {
    fn new() -> Self {
        Self {
            game_cluster: String::new(),
            general_cluster: String::new(),
            board_clusters: Vec::new(),
            cross_edges: String::new(),
            seen_game: HashSet::new(),
            seen_general: HashSet::new(),
            seen_board: HashSet::new(),
            game_ranks: BTreeMap::new(),
            general_ranks: Vec::new(),
            board_ranks: Vec::new(),
        }
    }

    #[inline]
    fn ptr_id<T>(r: &RefCell<T>) -> usize {
        (r as *const RefCell<T>) as usize
    }

    fn game_node_id<'a, H: Heuristic<'a>>(node: GameNodeRef<'a, H>) -> String {
        format!("g_{}", Self::ptr_id(node))
    }

    fn general_node_id<'a, H: Heuristic<'a>>(
        node: GeneralNodeRef<'a, H::CombinedEnc, H>,
    ) -> String {
        format!("gen_{}", Self::ptr_id(node))
    }

    fn board_node_id<'a, H: Heuristic<'a>>(node: BoardNodeRef<'a, H::CombinedEnc, H>) -> String {
        format!("b_{}", Self::ptr_id(node))
    }

    fn emit_game<'a, H: Heuristic<'a>>(&mut self, node: GameNodeRef<'a, H>) {
        let id = Self::ptr_id(node);
        if !self.seen_game.insert(id) {
            return;
        }

        let b = node.borrow();
        let node_name = Self::game_node_id(node);
        let visits = b.stats.visits;
        let side = b.state.game_state.side_to_move;
        let winprob = b.stats.eval.unwrap().score(side);
        let num_children = b.edges.len();
        let phantom_win = b
            .phantom_stats
            .eval
            .map(|e| e.score(side))
            .unwrap_or(f32::NAN);
        let ply = b.state.game_state.ply as usize;
        let border_color = match side {
            crate::core::side::Side::Yellow => "yellow",
            crate::core::side::Side::Blue => "blue",
        };
        let _ = writeln!(
            self.game_cluster,
            "  {} [label=\"Game\\nvisits={} win={:.3}\\nnum_children={} phantom={:.3}\", color={}];",
            node_name, visits, winprob, num_children, phantom_win, border_color
        );
        // Track rank groupings for game nodes by ply
        self.game_ranks
            .entry(ply)
            .or_default()
            .push(node_name.clone());

        // Traverse General and Boards without emitting cross edges
        let gen_ref = b.state.general_node;
        self.emit_general(gen_ref, 0);

        let b = node.borrow();
        for (i, board_ref) in b.state.board_nodes.iter().enumerate() {
            // Start Board traversal (per board) with depth 0
            self.emit_board(i, *board_ref, 0);
        }

        // Recurse into Game children (limit to top 4)
        for edge in b.edges.iter().take(4) {
            let child_ref = edge.child;
            let child_name = Self::game_node_id(child_ref);
            let _ = writeln!(self.game_cluster, "  {} -> {};", node_name, child_name);
            self.emit_game(child_ref);
        }
    }

    fn emit_general<'a, H: Heuristic<'a>>(
        &mut self,
        node: GeneralNodeRef<'a, H::CombinedEnc, H>,
        depth: usize,
    ) {
        let id = Self::ptr_id(node);
        if !self.seen_general.insert(id) {
            return;
        }
        let b = node.borrow();
        let node_name = Self::general_node_id(node);
        let visits = b.stats.visits;
        let side = b.state.side;
        let winprob = b.stats.eval.unwrap().score(side);
        let num_children = b.edges.len();
        let phantom_win = b
            .phantom_stats
            .eval
            .map(|e| e.score(side))
            .unwrap_or(f32::NAN);
        let border_color = match side {
            crate::core::side::Side::Yellow => "yellow",
            crate::core::side::Side::Blue => "blue",
        };
        let _ = writeln!(
            self.general_cluster,
            "  {} [label=\"General\\nvisits={} win={:.3}\\nnum_children={} phantom={:.3}\", color={}];",
            node_name, visits, winprob, num_children, phantom_win, border_color
        );
        // Track rank groupings by depth within General
        while self.general_ranks.len() <= depth {
            self.general_ranks.push(Vec::new());
        }
        self.general_ranks[depth].push(node_name.clone());
        // Recurse into General children (limit to top 4)
        for edge in b.edges.iter().take(4) {
            let child_ref = edge.child;
            let child_name = Self::general_node_id(child_ref);
            let _ = writeln!(self.general_cluster, "  {} -> {};", node_name, child_name);
            self.emit_general(child_ref, depth + 1);
        }
    }

    fn emit_board<'a, H: Heuristic<'a>>(
        &mut self,
        idx: usize,
        node: BoardNodeRef<'a, H::CombinedEnc, H>,
        depth: usize,
    ) {
        let id = Self::ptr_id(node);
        if !self.seen_board.insert(id) {
            return;
        }
        let b = node.borrow();
        let node_name = Self::board_node_id(node);
        let visits = b.stats.visits;
        let side = b.state.side_to_move;
        let winprob = b.stats.eval.unwrap().score(side);
        let num_children = b.edges.len();
        let phantom_win = b
            .phantom_stats
            .eval
            .map(|e| e.score(side))
            .unwrap_or(f32::NAN);
        let border_color = match side {
            crate::core::side::Side::Yellow => "yellow",
            crate::core::side::Side::Blue => "blue",
        };
        let _ = writeln!(
            self.board_cluster(idx),
            "  {} [label=\"Board {}\\nvisits={} win={:.3}\\nnum_children={} phantom={:.3}\", color={}];",
            node_name,
            idx,
            visits,
            winprob,
            num_children,
            phantom_win,
            border_color
        );
        // Track rank groupings by depth within this board subtree
        while self.board_ranks.len() <= idx {
            self.board_ranks.push(Vec::new());
        }
        while self.board_ranks[idx].len() <= depth {
            self.board_ranks[idx].push(Vec::new());
        }
        self.board_ranks[idx][depth].push(node_name.clone());
        for edge in b.edges.iter().take(4) {
            let child_ref = edge.child;
            let child_name = Self::board_node_id(child_ref);
            let _ = writeln!(
                self.board_cluster(idx),
                "  {} -> {};",
                node_name,
                child_name
            );
            self.emit_board(idx, child_ref, depth + 1);
        }
    }

    fn board_cluster(&mut self, idx: usize) -> &mut String {
        while self.board_clusters.len() <= idx {
            self.board_clusters.push(String::new());
        }
        &mut self.board_clusters[idx]
    }
}

pub fn export_search_tree<'a, H: Heuristic<'a>>(tree: &SearchTree<'a, H>) -> String {
    let mut gv = GraphvizBuilder::new();

    // Traverse starting from root game node
    gv.emit_game(tree.root);

    // Assemble DOT document with clusters and cross links
    let mut dot = String::new();
    dot.push_str("digraph SpookyMCTS {\n");
    dot.push_str("  graph [fontname=\"Helvetica\", compound=true];\n");
    dot.push_str("  node  [shape=box, fontname=\"Helvetica\"];\n");
    dot.push_str("  edge  [fontname=\"Helvetica\"];\n");
    dot.push_str("  rankdir=LR;\n\n");

    // Game cluster
    dot.push_str("  subgraph cluster_game {\n");
    dot.push_str("    label=\"Game Tree\";\n");
    dot.push_str("    color=\"#4C78A8\";\n");
    dot.push_str("    left_anchor [shape=point, width=0, height=0, label=\"\"];\n");
    dot.push_str(&gv.game_cluster);
    // Per-ply rank groups for Game
    for (_ply, nodes) in gv.game_ranks.iter() {
        if nodes.is_empty() {
            continue;
        }
        dot.push_str("    { rank=same; ");
        for n in nodes.iter() {
            dot.push_str(n);
            dot.push_str("; ");
        }
        dot.push_str("}\n");
    }
    dot.push_str("  }\n\n");

    // Right side container
    dot.push_str("  subgraph cluster_right {\n");
    dot.push_str("    label=\"Component Trees\";\n");
    dot.push_str("    color=\"#999999\";\n");
    dot.push_str("    right_anchor [shape=point, width=0, height=0, label=\"\"];\n");

    // General cluster
    dot.push_str("    subgraph cluster_general {\n");
    dot.push_str("      label=\"General\";\n");
    dot.push_str("      color=\"#F58518\";\n");
    dot.push_str("      gen_anchor [shape=point, width=0, height=0, label=\"\"];\n");
    dot.push_str(&gv.general_cluster.replace("\n", "\n      "));
    // Per-depth rank groups for General
    for nodes in gv.general_ranks.iter() {
        if nodes.is_empty() {
            continue;
        }
        dot.push_str("      { rank=same; ");
        for n in nodes.iter() {
            dot.push_str(n);
            dot.push_str("; ");
        }
        dot.push_str("}\n");
    }
    dot.push_str("    }\n\n");

    // Board clusters (one per board index)
    for (i, cluster) in gv.board_clusters.iter().enumerate() {
        if cluster.is_empty() {
            continue;
        }
        dot.push_str(&format!("    subgraph cluster_board_{} {{\n", i));
        dot.push_str(&format!("      label=\"Board {}\";\n", i));
        dot.push_str("      color=\"#54A24B\";\n");
        dot.push_str(&format!(
            "      board_{}_anchor [shape=point, width=0, height=0, label=\"\"];\n",
            i
        ));
        dot.push_str(&cluster.replace("\n", "\n      "));
        // Per-depth rank groups for this Board
        if i < gv.board_ranks.len() {
            for nodes in gv.board_ranks[i].iter() {
                if nodes.is_empty() {
                    continue;
                }
                dot.push_str("      { rank=same; ");
                for n in nodes.iter() {
                    dot.push_str(n);
                    dot.push_str("; ");
                }
                dot.push_str("}\n");
            }
        }
        dot.push_str("    }\n\n");
    }

    dot.push_str("  }\n\n");

    // Cross links
    dot.push_str(&gv.cross_edges);

    // Anchors to force left/right split and vertical stacking on right
    dot.push_str("  left_anchor -> right_anchor [style=invis, weight=100];\n");
    // Force all right-side region anchors into the same rank (same column in LR)
    dot.push_str("  { rank=same; right_anchor; gen_anchor");
    for i in 0..gv.board_clusters.len() {
        if gv.board_clusters[i].is_empty() {
            continue;
        }
        dot.push_str(&format!("; board_{}_anchor", i));
    }
    dot.push_str("; }\n");

    // Ensure all Game ranks precede right-side ranks by forcing earliest right nodes
    // to be strictly to the right of every Game ply representative.
    for (_ply, nodes) in gv.game_ranks.iter() {
        if let Some(game_rep) = nodes.first() {
            // General root rank representative (depth 0)
            if let Some(gen0) = gv.general_ranks.first() {
                if let Some(gen_rep) = gen0.first() {
                    dot.push_str(&format!(
                        "  {} -> {} [style=invis, weight=200, minlen=1];\n",
                        game_rep, gen_rep
                    ));
                }
            }
            // Each board's root rank representative (depth 0)
            for branks in gv.board_ranks.iter() {
                if let Some(b0) = branks.first() {
                    if let Some(board_rep) = b0.first() {
                        dot.push_str(&format!(
                            "  {} -> {} [style=invis, weight=200, minlen=1];\n",
                            game_rep, board_rep
                        ));
                    }
                }
            }
        }
    }

    dot.push_str("}\n");

    dot
}
