use std::collections::BTreeSet;
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
    let mut ids = BTreeSet::new();
    for action in actions {
        let id = action["id"].as_str().expect("action id");
        assert!(ids.insert(id), "duplicate action id: {id}");
        assert!(
            action["description"]
                .as_str()
                .is_some_and(|description| !description.trim().is_empty()),
            "action {id} must explain its behavior in the install preview"
        );
        assert!(
            !action["command"]
                .as_array()
                .expect("action argv")
                .is_empty(),
            "action {id} needs an argv command"
        );
    }
    assert_eq!(
        ids,
        BTreeSet::from([
            "choose-flok",
            "cleanup-flok",
            "cleanup-preview",
            "doctor",
            "open-flok",
        ])
    );

    let cleanup = actions
        .iter()
        .find(|action| action["id"].as_str() == Some("cleanup-preview"))
        .expect("cleanup preview action");
    let cleanup_command = cleanup["command"]
        .as_array()
        .expect("cleanup command")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert!(cleanup_command.contains(&"cleanup"));
    assert!(!cleanup_command.contains(&"--confirm"));

    let panes = plugin["panes"].as_array().expect("panes");
    assert_eq!(panes.len(), 2);
    for pane in panes {
        assert_eq!(
            pane["placement"].as_str(),
            Some("overlay"),
            "Herdr 0.7.5 accepts overlay, not the retired popup placement"
        );
    }

    for script in ["open-flok-picker.sh", "open-flok-cleanup.sh"] {
        let contents = fs::read_to_string(root.join("scripts").join(script)).expect("launcher");
        assert!(contents.contains("--placement overlay"));
        assert!(!contents.contains("--width"));
        assert!(!contents.contains("--height"));
    }
    let cleanup_launcher =
        fs::read_to_string(root.join("scripts/open-flok-cleanup.sh")).expect("cleanup launcher");
    assert!(cleanup_launcher.contains("HERDR_PLUGIN_CONTEXT_JSON=$HERDR_PLUGIN_CONTEXT_JSON"));
}
