use forge_session::services::{
    ManagedService, ServiceDefinitionError, ServiceName, ServicePlan, ServicePlanError,
    StartupRestartPolicy,
};

fn name(value: &str) -> ServiceName {
    ServiceName::new(value).unwrap()
}

fn service(
    value: &str,
    order: u16,
    dependencies: &[&str],
    restart_policy: StartupRestartPolicy,
) -> ManagedService {
    ManagedService::new(
        name(value),
        order,
        dependencies.iter().map(|dependency| name(dependency)),
        restart_policy,
    )
    .unwrap()
}

#[test]
fn explicit_startup_and_reverse_shutdown_order_are_stable() {
    let plan = ServicePlan::new([
        service("forge-world", 30, &["forge-core"], StartupRestartPolicy::Never),
        service("nyx", 20, &["forge-core"], StartupRestartPolicy::Never),
        service("forge-core", 10, &[], StartupRestartPolicy::Never),
    ])
    .unwrap();
    assert_eq!(
        plan.startup_order()
            .iter()
            .map(ServiceName::as_str)
            .collect::<Vec<_>>(),
        vec!["forge-core", "nyx", "forge-world"]
    );
    assert_eq!(
        plan.shutdown_order()
            .iter()
            .map(ServiceName::as_str)
            .collect::<Vec<_>>(),
        vec!["forge-world", "nyx", "forge-core"]
    );
}

#[test]
fn adding_unrelated_service_preserves_existing_relative_order() {
    let original = ServicePlan::new([
        service("forge-core", 10, &[], StartupRestartPolicy::Never),
        service("forge-world", 30, &[], StartupRestartPolicy::Never),
    ])
    .unwrap();
    let expanded = ServicePlan::new([
        service("forge-core", 10, &[], StartupRestartPolicy::Never),
        service("nyx", 20, &[], StartupRestartPolicy::Never),
        service("forge-world", 30, &[], StartupRestartPolicy::Never),
    ])
    .unwrap();
    let filtered: Vec<_> = expanded
        .startup_order()
        .iter()
        .filter(|value| value.as_str() != "nyx")
        .cloned()
        .collect();
    assert_eq!(filtered.as_slice(), original.startup_order());
}

#[test]
fn duplicate_name_is_rejected() {
    let error = ServicePlan::new([
        service("nyx", 10, &[], StartupRestartPolicy::Never),
        service("nyx", 20, &[], StartupRestartPolicy::Never),
    ])
    .unwrap_err();
    assert_eq!(error, ServicePlanError::DuplicateName(name("nyx")));
}

#[test]
fn duplicate_startup_rank_is_rejected() {
    let error = ServicePlan::new([
        service("forge-core", 10, &[], StartupRestartPolicy::Never),
        service("nyx", 10, &[], StartupRestartPolicy::Never),
    ])
    .unwrap_err();
    assert!(matches!(
        error,
        ServicePlanError::DuplicateStartupOrder { order: 10, .. }
    ));
}

#[test]
fn unknown_dependency_is_rejected() {
    let error = ServicePlan::new([service(
        "forge-world",
        20,
        &["missing"],
        StartupRestartPolicy::Never,
    )])
    .unwrap_err();
    assert_eq!(
        error,
        ServicePlanError::UnknownDependency {
            service: name("forge-world"),
            dependency: name("missing"),
        }
    );
}

#[test]
fn dependency_must_have_an_earlier_explicit_rank() {
    let error = ServicePlan::new([
        service("forge-world", 10, &["forge-core"], StartupRestartPolicy::Never),
        service("forge-core", 20, &[], StartupRestartPolicy::Never),
    ])
    .unwrap_err();
    assert!(matches!(
        error,
        ServicePlanError::DependencyOrder {
            service,
            dependency,
            ..
        } if service == name("forge-world") && dependency == name("forge-core")
    ));
}

#[test]
fn duplicate_and_self_dependencies_are_rejected_before_plan_creation() {
    assert_eq!(
        ManagedService::new(
            name("forge-world"),
            20,
            [name("forge-core"), name("forge-core")],
            StartupRestartPolicy::Never,
        )
        .unwrap_err(),
        ServiceDefinitionError::DuplicateDependency(name("forge-core"))
    );
    assert_eq!(
        ManagedService::new(
            name("forge-world"),
            20,
            [name("forge-world")],
            StartupRestartPolicy::Never,
        )
        .unwrap_err(),
        ServiceDefinitionError::SelfDependency(name("forge-world"))
    );
}

#[test]
fn service_names_are_canonical_lower_kebab_text() {
    for invalid in ["", "Nyx", "nyx_server", "-nyx", "nyx-", "nyx--host"] {
        assert!(ServiceName::new(invalid).is_err(), "accepted {invalid:?}");
    }
    assert_eq!(name("nyx-server").as_str(), "nyx-server");
}
