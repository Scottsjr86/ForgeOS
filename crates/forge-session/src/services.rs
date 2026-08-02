//! Deterministic managed-service definitions and startup plans.
//!
//! Service ordering is declared explicitly. Dependencies may constrain that order,
//! but filesystem discovery, map iteration, process names, and timing never do.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Maximum UTF-8 byte length of one stable managed-service name.
pub const MAX_SERVICE_NAME_BYTES: usize = 64;

/// Stable human-readable identity for one service inside a session plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceName(String);

impl ServiceName {
    /// Creates a validated lower-kebab service name.
    pub fn new(value: impl Into<String>) -> Result<Self, ServiceNameError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ServiceNameError::Empty);
        }
        if value.len() > MAX_SERVICE_NAME_BYTES {
            return Err(ServiceNameError::TooLong {
                actual: value.len(),
                maximum: MAX_SERVICE_NAME_BYTES,
            });
        }

        let bytes = value.as_bytes();
        if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
            return Err(ServiceNameError::BoundarySeparator);
        }

        for (index, byte) in bytes.iter().copied().enumerate() {
            if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
                return Err(ServiceNameError::InvalidByte { index, byte });
            }
        }
        if bytes.windows(2).any(|pair| pair == b"--") {
            return Err(ServiceNameError::RepeatedSeparator);
        }

        Ok(Self(value))
    }

    /// Stable service-name text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Invalid stable service name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceNameError {
    Empty,
    TooLong { actual: usize, maximum: usize },
    BoundarySeparator,
    RepeatedSeparator,
    InvalidByte { index: usize, byte: u8 },
}

impl fmt::Display for ServiceNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("service name is empty"),
            Self::TooLong { actual, maximum } => write!(
                formatter,
                "service name is {actual} bytes; maximum is {maximum}"
            ),
            Self::BoundarySeparator => {
                formatter.write_str("service name may not start or end with '-'")
            }
            Self::RepeatedSeparator => {
                formatter.write_str("service name may not contain repeated '-'")
            }
            Self::InvalidByte { index, byte } => write!(
                formatter,
                "service name byte {index} is invalid: 0x{byte:02x}"
            ),
        }
    }
}

impl std::error::Error for ServiceNameError {}

/// Restart behavior before a service reaches explicit readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupRestartPolicy {
    /// A failed start or readiness probe fails the session startup.
    Never,
    /// Retry failed start or readiness up to `max_restarts` times.
    OnFailure { max_restarts: u16 },
}

impl StartupRestartPolicy {
    /// Returns the next one-based attempt after `failed_attempt`, when permitted.
    pub fn next_attempt(self, failed_attempt: u16) -> Option<u16> {
        match self {
            Self::Never => None,
            Self::OnFailure { max_restarts } if failed_attempt <= max_restarts => {
                failed_attempt.checked_add(1)
            }
            Self::OnFailure { .. } => None,
        }
    }
}

/// Immutable definition of one service in a session plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedService {
    name: ServiceName,
    startup_order: u16,
    dependencies: Vec<ServiceName>,
    restart_policy: StartupRestartPolicy,
}

impl ManagedService {
    /// Creates one explicitly ordered service definition.
    pub fn new<I>(
        name: ServiceName,
        startup_order: u16,
        dependencies: I,
        restart_policy: StartupRestartPolicy,
    ) -> Result<Self, ServiceDefinitionError>
    where
        I: IntoIterator<Item = ServiceName>,
    {
        let mut dependencies: Vec<_> = dependencies.into_iter().collect();
        dependencies.sort();
        for pair in dependencies.windows(2) {
            if pair[0] == pair[1] {
                return Err(ServiceDefinitionError::DuplicateDependency(
                    pair[0].clone(),
                ));
            }
        }
        if dependencies.iter().any(|dependency| dependency == &name) {
            return Err(ServiceDefinitionError::SelfDependency(name));
        }

        Ok(Self {
            name,
            startup_order,
            dependencies,
            restart_policy,
        })
    }

    /// Stable service identity.
    pub fn name(&self) -> &ServiceName {
        &self.name
    }

    /// Explicit ascending startup rank.
    pub const fn startup_order(&self) -> u16 {
        self.startup_order
    }

    /// Dependencies that must report readiness before this service starts.
    pub fn dependencies(&self) -> &[ServiceName] {
        &self.dependencies
    }

    /// Restart policy for failed start and readiness attempts.
    pub const fn restart_policy(&self) -> StartupRestartPolicy {
        self.restart_policy
    }
}

/// Invalid individual service definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceDefinitionError {
    DuplicateDependency(ServiceName),
    SelfDependency(ServiceName),
}

impl fmt::Display for ServiceDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDependency(name) => {
                write!(formatter, "service dependency is duplicated: {name}")
            }
            Self::SelfDependency(name) => write!(formatter, "service depends on itself: {name}"),
        }
    }
}

impl std::error::Error for ServiceDefinitionError {}

/// Deterministic startup and shutdown plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePlan {
    services: BTreeMap<ServiceName, ManagedService>,
    startup: Vec<ServiceName>,
    shutdown: Vec<ServiceName>,
}

impl ServicePlan {
    /// Validates a complete explicitly ordered service graph.
    pub fn new<I>(services: I) -> Result<Self, ServicePlanError>
    where
        I: IntoIterator<Item = ManagedService>,
    {
        let services: Vec<_> = services.into_iter().collect();
        if services.is_empty() {
            return Err(ServicePlanError::Empty);
        }

        let mut by_name = BTreeMap::new();
        let mut by_order = BTreeMap::new();
        for service in services {
            if by_name.contains_key(service.name()) {
                return Err(ServicePlanError::DuplicateName(service.name().clone()));
            }
            if let Some(existing) = by_order.insert(service.startup_order(), service.name().clone())
            {
                return Err(ServicePlanError::DuplicateStartupOrder {
                    order: service.startup_order(),
                    first: existing,
                    second: service.name().clone(),
                });
            }
            by_name.insert(service.name().clone(), service);
        }

        for service in by_name.values() {
            for dependency in service.dependencies() {
                let dependency_service = by_name.get(dependency).ok_or_else(|| {
                    ServicePlanError::UnknownDependency {
                        service: service.name().clone(),
                        dependency: dependency.clone(),
                    }
                })?;
                if dependency_service.startup_order() >= service.startup_order() {
                    return Err(ServicePlanError::DependencyOrder {
                        service: service.name().clone(),
                        service_order: service.startup_order(),
                        dependency: dependency.clone(),
                        dependency_order: dependency_service.startup_order(),
                    });
                }
            }
        }

        let startup: Vec<_> = by_order.into_values().collect();
        let shutdown = startup.iter().rev().cloned().collect();
        Ok(Self {
            services: by_name,
            startup,
            shutdown,
        })
    }

    /// Service definitions indexed by stable name.
    pub fn services(&self) -> &BTreeMap<ServiceName, ManagedService> {
        &self.services
    }

    /// Explicit startup sequence.
    pub fn startup_order(&self) -> &[ServiceName] {
        &self.startup
    }

    /// Explicit reverse-dependency-safe shutdown sequence.
    pub fn shutdown_order(&self) -> &[ServiceName] {
        &self.shutdown
    }

    /// Looks up one service definition.
    pub fn service(&self, name: &ServiceName) -> Option<&ManagedService> {
        self.services.get(name)
    }

    /// True when every declared dependency appears in `ready`.
    pub fn dependencies_ready(
        &self,
        name: &ServiceName,
        ready: &BTreeSet<ServiceName>,
    ) -> bool {
        self.service(name)
            .map(|service| {
                service
                    .dependencies()
                    .iter()
                    .all(|dependency| ready.contains(dependency))
            })
            .unwrap_or(false)
    }
}

/// Invalid complete service plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServicePlanError {
    Empty,
    DuplicateName(ServiceName),
    DuplicateStartupOrder {
        order: u16,
        first: ServiceName,
        second: ServiceName,
    },
    UnknownDependency {
        service: ServiceName,
        dependency: ServiceName,
    },
    DependencyOrder {
        service: ServiceName,
        service_order: u16,
        dependency: ServiceName,
        dependency_order: u16,
    },
}

impl fmt::Display for ServicePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("service plan is empty"),
            Self::DuplicateName(name) => write!(formatter, "duplicate service name: {name}"),
            Self::DuplicateStartupOrder {
                order,
                first,
                second,
            } => write!(
                formatter,
                "startup order {order} is shared by {first} and {second}"
            ),
            Self::UnknownDependency {
                service,
                dependency,
            } => write!(formatter, "service {service} depends on unknown {dependency}"),
            Self::DependencyOrder {
                service,
                service_order,
                dependency,
                dependency_order,
            } => write!(
                formatter,
                "service {service} at order {service_order} depends on {dependency} at non-earlier order {dependency_order}"
            ),
        }
    }
}

impl std::error::Error for ServicePlanError {}
