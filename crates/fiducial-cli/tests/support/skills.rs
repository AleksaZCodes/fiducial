//! Test support: state the skills a product needs as installed, the way the
//! `skills` CLI leaves them.
//!
//! The CLI installs from the network, which a unit-sized test must not do, and
//! what it writes is simple and documented: each skill's folder under
//! `.agents/skills/` and an entry in `skills-lock.json`. The set to write is read
//! from `.fiducial/agent-skills.json`, which `fid` derives from the product's
//! capabilities, so this stays right as capabilities name more skills.

use std::path::Path;

/// Install, as the CLI would, every skill the product's capabilities require.
#[allow(dead_code)]
pub fn install_required_skills(root: &Path) {
    let gate = std::fs::read_to_string(root.join(".fiducial/agent-skills.json"))
        .map(|s| serde_json::from_str::<serde_json::Value>(&s).unwrap())
        .unwrap_or_default();
    let mut entries = serde_json::Map::new();
    if let Ok(old) = std::fs::read_to_string(root.join("skills-lock.json")) {
        let v: serde_json::Value = serde_json::from_str(&old).unwrap();
        entries = v["skills"].as_object().cloned().unwrap_or_default();
    }
    for entry in gate["skills"].as_array().into_iter().flatten() {
        for name in entry["skills"].as_array().into_iter().flatten() {
            let name = name.as_str().unwrap();
            let dir = root.join(".agents/skills").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: a test double\n---\n"),
            )
            .unwrap();
            entries.insert(name.to_string(), serde_json::json!({ "source": "test" }));
        }
    }
    std::fs::write(
        root.join("skills-lock.json"),
        serde_json::to_string_pretty(&serde_json::json!({ "version": 2, "skills": entries }))
            .unwrap(),
    )
    .unwrap();
}
