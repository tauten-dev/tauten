//! Build the structural-check model of a workspace from disk.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, PackageId};
use tauten_check::structure::{ManifestLint, Member, SourceFile, Workspace};

/// A loaded workspace: cargo metadata plus the check model.
pub struct Loaded {
    pub metadata: Metadata,
    pub model: Workspace,
}

impl Loaded {
    pub fn root(&self) -> &Path {
        self.metadata.workspace_root.as_std_path()
    }

    pub fn package_name(&self, id: &PackageId) -> Option<&str> {
        self.metadata
            .packages
            .iter()
            .find(|p| &p.id == id)
            .map(|p| p.name.as_str())
    }
}

pub fn load(manifest_path: Option<&Path>, assume_tier: &[String]) -> Result<Loaded> {
    let assumed = assume_tier
        .iter()
        .map(|spec| {
            spec.split_once('=')
                .map(|(name, tier)| (name.to_owned(), tier.to_owned()))
                .with_context(|| format!("`--assume-tier {spec}` must look like crate=tier"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut cmd = MetadataCommand::new();
    if let Some(path) = manifest_path {
        cmd.manifest_path(path);
    }
    let metadata = cmd.no_deps().exec().context("running `cargo metadata`")?;
    let root = metadata.workspace_root.as_std_path().to_path_buf();
    let workspace_lints = read_workspace_lints(&root)?;
    let members: Vec<&Package> = metadata.workspace_packages();
    let names: Vec<&str> = members.iter().map(|p| p.name.as_str()).collect();
    let mut model = Workspace {
        members: members
            .iter()
            .map(|p| member(&root, p, &names, &workspace_lints))
            .collect::<Result<_>>()?,
    };
    for m in &mut model.members {
        let named = assumed.iter().find(|(name, _)| name == &m.name);
        let wildcard = assumed
            .iter()
            .find(|(name, _)| name == "*")
            .filter(|_| m.tier_raw.is_none());
        if let Some((_, tier)) = named.or(wildcard) {
            m.tier_raw = Some(tier.clone());
        }
    }
    Ok(Loaded { metadata, model })
}

fn member(
    root: &Path,
    package: &Package,
    names: &[&str],
    workspace_lints: &[ManifestLint],
) -> Result<Member> {
    let meta = package.metadata.get("tauten");
    let tier_raw = meta
        .and_then(|m| m.get("tier"))
        .and_then(|t| t.as_str())
        .map(str::to_owned);
    let kernel_dep_allowlist = meta
        .and_then(|m| m.get("kernel-deps"))
        .and_then(|d| d.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let effects_required =
        meta.and_then(|m| m.get("effects")).and_then(|e| e.as_str()) == Some("required");
    let normal = package
        .dependencies
        .iter()
        .filter(|d| d.kind == DependencyKind::Normal);
    let (workspace_deps, external_deps): (Vec<String>, Vec<String>) = {
        let all: Vec<String> = normal.map(|d| d.name.clone()).collect();
        all.into_iter()
            .partition(|name| names.contains(&name.as_str()))
    };
    let manifest_path = package.manifest_path.as_std_path();
    let manifest_lints = read_package_lints(manifest_path, workspace_lints)?;
    let crate_roots: Vec<PathBuf> = package
        .targets
        .iter()
        .filter(|t| t.is_lib() || t.is_bin() || t.is_proc_macro())
        .map(|t| t.src_path.as_std_path().to_path_buf())
        .collect();
    let package_dir = manifest_path.parent().unwrap_or(root);
    let mut sources = Vec::new();
    collect_sources(root, &package_dir.join("src"), &crate_roots, &mut sources)?;
    Ok(Member {
        name: package.name.to_string(),
        tier_raw,
        manifest: relative(root, manifest_path),
        workspace_deps,
        external_deps,
        kernel_dep_allowlist,
        manifest_lints,
        sources,
        effects_required,
    })
}

fn collect_sources(
    root: &Path,
    dir: &Path,
    crate_roots: &[PathBuf],
    out: &mut Vec<SourceFile>,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_sources(root, &path, crate_roots, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            out.push(SourceFile {
                path: relative(root, &path),
                is_crate_root: crate_roots.contains(&path),
                text,
            });
        }
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read_toml(path: &Path) -> Result<toml::Table> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    text.parse()
        .with_context(|| format!("parsing {}", path.display()))
}

fn read_workspace_lints(root: &Path) -> Result<Vec<ManifestLint>> {
    let table = read_toml(&root.join("Cargo.toml"))?;
    Ok(table
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .map(lints_from)
        .unwrap_or_default())
}

fn read_package_lints(
    manifest: &Path,
    workspace_lints: &[ManifestLint],
) -> Result<Vec<ManifestLint>> {
    let table = read_toml(manifest)?;
    let Some(lints) = table.get("lints") else {
        return Ok(Vec::new());
    };
    if lints.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        return Ok(workspace_lints.to_vec());
    }
    Ok(lints_from(lints))
}

/// Flatten `[lints.<tool>] name = level | { level = ... }` into lint names.
fn lints_from(lints: &toml::Value) -> Vec<ManifestLint> {
    let mut out = Vec::new();
    let Some(tools) = lints.as_table() else {
        return out;
    };
    for (tool, entries) in tools {
        let Some(entries) = entries.as_table() else {
            continue;
        };
        for (name, value) in entries {
            let level = value
                .as_str()
                .or_else(|| value.get("level").and_then(toml::Value::as_str))
                .unwrap_or_default();
            let name = name.replace('-', "_");
            let name = if tool == "rust" {
                name
            } else {
                format!("{tool}::{name}")
            };
            out.push(ManifestLint {
                name,
                level: level.to_owned(),
            });
        }
    }
    out
}
