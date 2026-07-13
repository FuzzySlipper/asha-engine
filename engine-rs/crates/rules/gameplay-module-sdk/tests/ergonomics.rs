use gameplay_module_sdk::*;

fn descriptor(name: &str) -> String {
    format!("ergonomics.fixture.{name};canonical-json-v1")
}

fn contract(name: &str) -> GameplayContractRef {
    gameplay_contract("ergonomics.fixture", name, 1, &descriptor(name))
}

fn selector() -> GameplayHeaderSelector {
    GameplayHeaderSelector {
        source: None,
        target: None,
        scope: None,
        required_tags: Vec::new(),
    }
}

fn invocation(id: &str) -> GameplayModuleInvocationTopology {
    GameplayModuleInvocationTopology::observe(
        format!("{id}.subscription"),
        id,
        contract("input"),
        contract("output"),
        selector(),
        4,
        2,
        1_024,
    )
}

#[test]
fn one_authored_read_derives_manifest_provider_and_runtime_topology() {
    let authored = invocation("ergonomics.fixture.observe").read(gameplay_session_state_read(
        "counter-state",
        contract("counter-view"),
        "provider.ergonomics-fixture",
        vec!["amount".to_owned()],
        "single-state",
    ));
    let topology =
        GameplayDerivedModuleTopology::derive("ergonomics.fixture.module", vec![authored]).unwrap();

    assert_eq!(topology.subscriptions().len(), 1);
    assert_eq!(topology.invocations().len(), 1);
    assert_eq!(topology.read_views().len(), 1);
    assert_eq!(topology.read_view_providers().len(), 1);
    assert_eq!(topology.declared_reads().len(), 1);
    let requirement = &topology.invocations()[0].read_requirements[0];
    let request = &topology.declared_reads()[0].requests[0];
    assert_eq!(requirement.request_id, request.request_id);
    assert_eq!(requirement.view, request.view);
    assert_eq!(topology.read_views()[0].view, request.view);
    assert_eq!(topology.read_views()[0].fields, request.fields);
    assert_eq!(
        topology.read_view_providers()[0].provider_id,
        "provider.ergonomics-fixture"
    );
}

#[test]
fn duplicate_invocations_requests_and_conflicting_views_fail_closed() {
    let duplicate = invocation("ergonomics.fixture.observe");
    assert!(matches!(
        GameplayDerivedModuleTopology::derive(
            "ergonomics.fixture.module",
            vec![duplicate.clone(), duplicate]
        ),
        Err(GameplayModuleTopologyError::DuplicateInvocation(_))
    ));

    let duplicate_read = gameplay_session_state_read(
        "counter-state",
        contract("counter-view"),
        "provider.ergonomics-fixture",
        vec!["amount".to_owned()],
        "single-state",
    );
    assert!(matches!(
        GameplayDerivedModuleTopology::derive(
            "ergonomics.fixture.module",
            vec![invocation("ergonomics.fixture.observe")
                .read(duplicate_read.clone())
                .read(duplicate_read)]
        ),
        Err(GameplayModuleTopologyError::DuplicateReadRequest { .. })
    ));

    let first = invocation("ergonomics.fixture.first").read(gameplay_session_state_read(
        "counter-state",
        contract("counter-view"),
        "provider.ergonomics-fixture",
        vec!["amount".to_owned()],
        "single-state",
    ));
    let second = invocation("ergonomics.fixture.second").read(gameplay_session_state_read(
        "counter-state",
        contract("counter-view"),
        "provider.foreign",
        vec!["amount".to_owned()],
        "single-state",
    ));
    assert!(matches!(
        GameplayDerivedModuleTopology::derive("ergonomics.fixture.module", vec![first, second]),
        Err(GameplayModuleTopologyError::ConflictingReadView(_))
    ));
}

#[test]
fn decision_invocations_cannot_smuggle_observe_subscription_shape() {
    let malformed = GameplayModuleInvocationTopology::decision(
        "ergonomics.fixture.bad-observe",
        GameplayInvocationFamily::Observe,
        contract("input"),
        contract("output"),
        1,
        1_024,
    );
    assert!(matches!(
        GameplayDerivedModuleTopology::derive("ergonomics.fixture.module", vec![malformed]),
        Err(GameplayModuleTopologyError::MissingObserveSubscription(_))
    ));
}
