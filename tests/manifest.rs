use std::fs;
use std::path::PathBuf;

#[test]
fn plugin_manifest_is_a_transparent_install_and_action_contract() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let plugin: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("herdr-plugin.toml")).expect("plugin manifest"),
    )
    .expect("valid plugin TOML");
    let cargo: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.toml")).expect("Cargo manifest"))
            .expect("valid Cargo TOML");

    assert_eq!(plugin["id"].as_str(), Some("m-mohamed.sheprd"));
    assert_eq!(plugin["name"].as_str(), Some("Sheprd"));
    assert_eq!(plugin["min_herdr_version"].as_str(), Some("0.7.5"));
    assert_eq!(
        plugin["version"].as_str(),
        cargo["package"]["version"].as_str(),
        "plugin and crate versions must remain identical"
    );
    assert_eq!(
        plugin["platforms"]
            .as_array()
            .expect("platform list")
            .iter()
            .filter_map(toml::Value::as_str)
            .collect::<Vec<_>>(),
        ["linux", "macos"]
    );
    assert_eq!(
        plugin["build"][0]["command"]
            .as_array()
            .expect("build argv")
            .iter()
            .filter_map(toml::Value::as_str)
            .collect::<Vec<_>>(),
        ["bash", "scripts/install-plugin.sh"]
    );

    let actions = plugin["actions"].as_array().expect("actions");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["id"].as_str(), Some("doctor"));
    assert!(actions[0]["description"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(plugin.get("panes").is_none());
}
