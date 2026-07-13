use asha_gameplay_module_conformance::{
    run_gameplay_module_conformance, GameplayModuleConformanceCase,
};
use asha_gameplay_module_fixture::{
    composition as build_composition, conformance_needs_manifest_json,
    conformance_reachable_surfaces, root_event, trigger_entered_event,
};
use asha_gameplay_module_sdk::{GameplayStaticComposition, GameplayStaticCompositionError};

fn composition() -> Result<GameplayStaticComposition, GameplayStaticCompositionError> {
    Ok(build_composition(4))
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let json_path = match arguments.as_slice() {
        [] => None,
        [flag, path] if flag == "--json" => Some(path),
        _ => {
            eprintln!("usage: conformance [--json <report-path>]");
            std::process::exit(2);
        }
    };
    let project_bundle_json = include_str!("../../project/gameplay-project.json").to_owned();
    let report = run_gameplay_module_conformance(GameplayModuleConformanceCase {
        project_bundle_json,
        consumer_needs_manifest_json: conformance_needs_manifest_json(),
        reachable_surfaces: conformance_reachable_surfaces(),
        composition,
        events: vec![root_event(7), trigger_entered_event(10, 20)],
    })
    .expect("conformance runner executes");
    if let Some(path) = json_path {
        std::fs::write(path, report.to_pretty_json().expect("report serializes"))
            .expect("conformance report writes");
    }
    print!("{}", report.trace);
    if !report.valid {
        eprintln!("{}", report.to_pretty_json().expect("report serializes"));
        std::process::exit(1);
    }
}
