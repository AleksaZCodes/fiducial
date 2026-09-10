//! `fid graph [--format text|dot|json]` — emit the facts → pipelines → artifacts DAG.
//!
//! Reads `pipelines/*.toml` from the product root and emits the dependency graph
//! in the requested format. Consumed by `fid dash` (Phase 16) and by agents
//! reasoning about what needs to be re-derived.

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::{Config, CONFIG_FILE},
    pipeline::{self, Pipeline},
};

// ── Graph types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GraphNode {
    id: String,
    kind: &'static str,
    label: String,
}

#[derive(Debug, Serialize)]
struct GraphEdge {
    from: String,
    to: String,
}

#[derive(Debug, Serialize)]
struct Graph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(format: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let config = Config::load(&root.join(CONFIG_FILE))?;

    let pipelines = pipeline::discover(&root, &config)?;

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    for pipeline in &pipelines {
        let pipeline_id = format!("pipeline:{}", pipeline.name);
        nodes.push(GraphNode {
            id: pipeline_id.clone(),
            kind: "pipeline",
            label: pipeline.name.clone(),
        });

        for out in &pipeline.outputs {
            let artifact_id = format!("artifact:{out}");
            if !nodes.iter().any(|n| n.id == artifact_id) {
                nodes.push(GraphNode {
                    id: artifact_id.clone(),
                    kind: "artifact",
                    label: out.clone(),
                });
            }
            edges.push(GraphEdge {
                from: pipeline_id.clone(),
                to: artifact_id,
            });
        }
    }

    let graph = Graph { nodes, edges };

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&graph)?;
            println!("{json}");
        }
        "dot" => emit_dot(&graph),
        _ => emit_text(&pipelines),
    }

    Ok(())
}

// ── Format emitters ───────────────────────────────────────────────────────────

fn emit_text(pipelines: &[Pipeline]) {
    if pipelines.is_empty() {
        println!("(no pipelines declared)");
        println!("Declare pipelines in `pipelines/*.toml` or enable [spine] in fiducial.toml.");
        return;
    }
    for p in pipelines {
        println!("pipeline: {} ({})", p.name, p.executor);
        for out in &p.outputs {
            println!("  → artifact: {out}");
        }
    }
}

fn emit_dot(graph: &Graph) {
    println!("digraph fiducial {{");
    println!("  rankdir=LR;");
    for node in &graph.nodes {
        let shape = if node.kind == "pipeline" {
            "box"
        } else {
            "ellipse"
        };
        println!("  {:?} [label={:?} shape={shape}];", node.id, node.label);
    }
    for edge in &graph.edges {
        println!("  {:?} -> {:?};", edge.from, edge.to);
    }
    println!("}}");
}
