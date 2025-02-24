// src/dependency.rs
use crate::types::FileEntry;
use anyhow::{anyhow, Result};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// Extract dependencies from a file based on its language
fn extract_dependencies(file_path: &Path, content: &str) -> Vec<String> {
    let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "rs" => extract_rust_dependencies(content),
        "py" => extract_python_dependencies(content),
        "js" | "ts" => extract_js_ts_dependencies(content),
        "java" => extract_java_dependencies(content),
        _ => vec![], // Unsupported file types return empty list
    }
}

// Extract Rust dependencies (e.g., `use` statements)
fn extract_rust_dependencies(content: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    for line in content.lines() {
        if line.trim().starts_with("use ") {
            if let Some(dep) = line.split("use ").nth(1) {
                let dep = dep.trim().trim_end_matches(';').to_string();
                // Convert module path to potential file path (simplified)
                let dep_path = dep.replace("::", "/") + ".rs";
                dependencies.push(dep_path);
            }
        }
    }
    dependencies
}

// Extract Python dependencies (e.g., `import` statements)
fn extract_python_dependencies(content: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    for line in content.lines() {
        if line.trim().starts_with("import ") || line.trim().starts_with("from ") {
            if let Some(dep) = line.split_whitespace().nth(1) {
                let dep_path = dep.replace(".", "/") + ".py";
                dependencies.push(dep_path);
            }
        }
    }
    dependencies
}

// Extract JavaScript/TypeScript dependencies (e.g., `import` statements)
fn extract_js_ts_dependencies(content: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    for line in content.lines() {
        if line.trim().starts_with("import ") {
            if let Some(dep) = line.split(['"', '\'']).nth(1) {
                let dep_path = if dep.ends_with(".js") || dep.ends_with(".ts") {
                    dep.to_string()
                } else {
                    dep.to_string() + ".js" // Default to .js if no extension
                };
                dependencies.push(dep_path);
            }
        }
    }
    dependencies
}

// Extract Java dependencies (e.g., `import` statements)
fn extract_java_dependencies(content: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    for line in content.lines() {
        if line.trim().starts_with("import ") {
            if let Some(dep) = line.split("import ").nth(1) {
                let dep = dep.trim().trim_end_matches(';');
                let dep_path = dep.replace(".", "/") + ".java";
                dependencies.push(dep_path);
            }
        }
    }
    dependencies
}

// Build the dependency graph
fn build_dependency_graph(files: &[FileEntry]) -> Result<DiGraph<PathBuf, ()>> {
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();

    // Add all files as nodes
    for file in files {
        let path = PathBuf::from(&file.rel_path);
        let index = graph.add_node(path.clone());
        node_indices.insert(path, index);
    }

    // Add edges based on dependencies
    for file in files {
        let path = PathBuf::from(&file.rel_path);
        let dependencies = extract_dependencies(&path, &file.content);
        if let Some(&file_index) = node_indices.get(&path) {
            for dep in dependencies {
                let dep_path = PathBuf::from(dep);
                if let Some(&dep_index) = node_indices.get(&dep_path) {
                    graph.add_edge(file_index, dep_index, ());
                }
            }
        }
    }

    Ok(graph)
}

// Sort files by dependency using topological sort
pub fn sort_files_by_dependency(files: &[FileEntry]) -> Result<Vec<FileEntry>> {
    let graph = build_dependency_graph(files)?;
    let sorted_indices =
        toposort(&graph, None).map_err(|_| anyhow!("Cycle detected in dependency graph"))?;

    let sorted_files: Vec<FileEntry> = sorted_indices
        .into_iter()
        .map(|index| {
            let path = &graph[index];
            files
                .iter()
                .find(|f| PathBuf::from(&f.rel_path) == *path)
                .unwrap()
                .clone()
        })
        .collect();

    Ok(sorted_files)
}
