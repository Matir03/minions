use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write as _;

use crate::ai::captain::node::BoardNodeRef;
use crate::ai::game::GameNodeRef;
use crate::ai::general::GeneralNodeRef;
use crate::ai::search::SearchTree;

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
        }
    }

    #[inline]
    fn ptr_id<T>(r: &RefCell<T>) -> usize {
        (r as *const RefCell<T>) as usize
    }

    fn game_node_id(node: GameNodeRef) -> String {
        format!("g_{}", Self::ptr_id(node))
    }

    fn general_node_id(node: GeneralNodeRef) -> String {
        format!("gen_{}", Self::ptr_id(node))
    }

    fn board_node_id(node: BoardNodeRef) -> String {
        format!("b_{}", Self::ptr_id(node))
    }

    fn emit_game(&mut self, node: GameNodeRef) {
        let id = Self::ptr_id(node);
        if !self.seen_game.insert(id) {
            return;
        }

        let b = node.borrow();
        let node_name = Self::game_node_id(node);
        let visits = b.stats.visits;
        let winprob = b.stats.eval.winprob;
        let side = b.state.game_state.side_to_move;
        let ply = b.state.game_state.ply;
        let _ = writeln!(
            self.game_cluster,
            "  {} [label=\"Game\\nvisits={} win={:.3}\\nside={:?} ply={}\"];",
            node_name, visits, winprob, side, ply
        );

        // Link to General and Board nodes (cross edges)
        let gen_ref = b.state.general_node;
        let gen_name = Self::general_node_id(gen_ref);
        let _ = writeln!(
            self.cross_edges,
            "  {} -> {} [style=dashed,label=\"general\"];",
            node_name, gen_name
        );
        self.emit_general(gen_ref);

        let b = node.borrow();
        for (i, board_ref) in b.state.board_nodes.iter().enumerate() {
            let board_name = Self::board_node_id(*board_ref);
            let _ = writeln!(
                self.cross_edges,
                "  {} -> {} [style=dashed,label=\"board {}\"];",
                node_name, board_name, i
            );
            self.emit_board(i, *board_ref);
        }

        // Recurse into Game children
        for edge in b.edges.iter() {
            let child_ref = edge.child;
            let child_name = Self::game_node_id(child_ref);
            let _ = writeln!(self.game_cluster, "  {} -> {};", node_name, child_name);
            self.emit_game(child_ref);
        }
    }

    fn emit_general(&mut self, node: GeneralNodeRef) {
        let id = Self::ptr_id(node);
        if !self.seen_general.insert(id) {
            return;
        }
        let b = node.borrow();
        let node_name = Self::general_node_id(node);
        let visits = b.stats.visits;
        let winprob = b.stats.eval.winprob;
        let side = b.state.side;
        let _ = writeln!(
            self.general_cluster,
            "  {} [label=\"General\\nvisits={} win={:.3}\\nside={:?}\"];",
            node_name, visits, winprob, side
        );
        for edge in b.edges.iter() {
            let child_ref = edge.child;
            let child_name = Self::general_node_id(child_ref);
            let _ = writeln!(self.general_cluster, "  {} -> {};", node_name, child_name);
            self.emit_general(child_ref);
        }
    }

    fn emit_board(&mut self, idx: usize, node: BoardNodeRef) {
        let id = Self::ptr_id(node);
        if !self.seen_board.insert(id) {
            return;
        }
        let b = node.borrow();
        let node_name = Self::board_node_id(node);
        let visits = b.stats.visits;
        let winprob = b.stats.eval.winprob;
        let side = b.state.side_to_move;
        let _ = writeln!(
            self.board_cluster(idx),
            "  {} [label=\"Board {}\\nvisits={} win={:.3}\\nside={:?}\"];",
            node_name, idx, visits, winprob, side
        );
        for edge in b.edges.iter() {
            let child_ref = edge.child;
            let child_name = Self::board_node_id(child_ref);
            let _ = writeln!(self.board_cluster(idx), "  {} -> {};", node_name, child_name);
            self.emit_board(idx, child_ref);
        }
    }

    fn board_cluster(&mut self, idx: usize) -> &mut String {
        while self.board_clusters.len() <= idx {
            self.board_clusters.push(String::new());
        }
        &mut self.board_clusters[idx]
    }
}

pub fn export_search_tree<'a>(tree: &SearchTree<'a>) -> String {
    let mut gv = GraphvizBuilder::new();

    // Traverse starting from root game node
    gv.emit_game(tree.root);

    // Assemble DOT document with clusters and cross links
    let mut dot = String::new();
    dot.push_str("digraph SpookyMCTS {\n");
    dot.push_str("  graph [fontname=\"Helvetica\"];\n");
    dot.push_str("  node  [shape=box, fontname=\"Helvetica\"];\n");
    dot.push_str("  edge  [fontname=\"Helvetica\"];\n");
    dot.push_str("  rankdir=LR;\n\n");

    // Game cluster
    dot.push_str("  subgraph cluster_game {\n");
    dot.push_str("    label=\"Game Tree\";\n");
    dot.push_str("    color=\"#4C78A8\";\n");
    dot.push_str(&gv.game_cluster);
    dot.push_str("  }\n\n");

    // General cluster
    dot.push_str("  subgraph cluster_general {\n");
    dot.push_str("    label=\"General Trees\";\n");
    dot.push_str("    color=\"#F58518\";\n");
    dot.push_str(&gv.general_cluster);
    dot.push_str("  }\n\n");

    // Board clusters (one per board index)
    for (i, cluster) in gv.board_clusters.iter().enumerate() {
        if cluster.is_empty() { continue; }
        dot.push_str(&format!("  subgraph cluster_board_{} {{\n", i));
        dot.push_str(&format!("    label=\"Board {}\";\n", i));
        dot.push_str("    color=\"#54A24B\";\n");
        dot.push_str(cluster);
        dot.push_str("  }\n\n");
    }

    // Cross links
    dot.push_str(&gv.cross_edges);

    dot.push_str("}\n");

    dot
}
