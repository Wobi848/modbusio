//! Every route of the web API is described in docs/api.md, and every simulator in README.md.

#[test]
fn every_route_is_in_the_api_description() {
    let router = std::fs::read_to_string("src/server/mod.rs").expect("router");
    let doc = std::fs::read_to_string("docs/api.md").expect("api.md");
    let routes: Vec<&str> = router
        .split(".route(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect();
    assert!(routes.len() > 30, "found only {} routes", routes.len());
    for route in routes {
        assert!(
            doc.contains(&format!("{route}`")) || doc.contains(&format!("{route}?")),
            "docs/api.md does not describe {route}"
        );
    }
}

#[test]
fn every_simulator_is_in_the_readme() {
    let readme = std::fs::read_to_string("README.md").expect("README.md");
    for entry in std::fs::read_dir("examples").expect("examples") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .file_stem()
                .expect("name")
                .to_string_lossy()
                .to_string();
            assert!(readme.contains(&name), "README.md does not describe {name}");
        }
    }
}
