//! Enterprise Modular BDD Architecture Example
//! 
//! This example demonstrates how to build scalable BDD test suites using
//! modular step definitions that can be owned by different teams and
//! composed into comprehensive test collections.
//!
//! Run with: `cargo run --example modular_enterprise_bdd`

use cucumber::{World, step::{Collection, StepBuilder, compose_step_builders}, step_builder};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use cucumber::step::Context;

/// Enterprise test world that tracks state across domains
#[derive(Debug, Default, World)]
pub struct EnterpriseWorld {
    /// Service health status
    pub services: HashMap<String, bool>,
    /// User authentication state  
    pub users: HashMap<String, Value>,
    /// Cryptographic keys created during tests
    pub keys: HashMap<String, Value>,
    /// Audit events for compliance verification
    pub audit_events: Vec<Value>,
    /// License and feature gate status
    pub license_info: Option<Value>,
}

impl EnterpriseWorld {
    pub fn add_service(&mut self, name: &str, healthy: bool) {
        self.services.insert(name.to_string(), healthy);
    }
    
    pub fn authenticate_user(&mut self, user: &str, role: &str) {
        self.users.insert(user.to_string(), json!({
            "name": user,
            "role": role,
            "authenticated": true
        }));
    }
    
    pub fn create_key(&mut self, key_id: &str, key_type: &str) {
        self.keys.insert(key_id.to_string(), json!({
            "id": key_id,
            "type": key_type,
            "created": true
        }));
    }
    
    pub fn log_audit_event(&mut self, event: Value) {
        self.audit_events.push(event);
    }
    
    pub fn set_license(&mut self, edition: &str, features: Vec<&str>) {
        self.license_info = Some(json!({
            "edition": edition,
            "features": features
        }));
    }
}

// =============================================================================
// INFRASTRUCTURE TEAM - Service Management & Health Monitoring
// =============================================================================

/// Infrastructure team owns service lifecycle and health monitoring steps
pub struct InfrastructureSteps;

impl StepBuilder<EnterpriseWorld> for InfrastructureSteps {
    fn register_steps(collection: Collection<EnterpriseWorld>) -> Collection<EnterpriseWorld> {
        collection
            .given(None, Regex::new(r"the vault service is running").unwrap(), vault_service_running)
            .given(None, Regex::new(r#"service "([^"]+)" is healthy"#).unwrap(), service_is_healthy)
            .when(None, Regex::new(r"checking the health endpoint").unwrap(), check_health_endpoint)
            .when(None, Regex::new(r#"monitoring service "([^"]+)""#).unwrap(), monitor_service)
            .then(None, Regex::new(r"the service should respond with healthy status").unwrap(), should_be_healthy)
            .then(None, Regex::new(r"all services should be operational").unwrap(), all_services_operational)
    }
    
    fn domain_name() -> &'static str {
        "Infrastructure & Service Management"
    }
}

fn vault_service_running(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🏗️ Infrastructure: Vault service is running");
        world.add_service("vault", true);
        world.add_service("database", true);
        world.add_service("network", true);
    })
}

fn service_is_healthy(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let service_name = &ctx.matches[1].1;
        println!("🏗️ Infrastructure: Service '{}' is healthy", service_name);
        world.add_service(service_name, true);
    })
}

fn check_health_endpoint(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🏗️ Infrastructure: Checking health endpoint");
        // Simulate health check
        world.log_audit_event(json!({
            "action": "health_check",
            "result": "success",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn monitor_service(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let service_name = &ctx.matches[1].1;
        println!("🏗️ Infrastructure: Monitoring service '{}'", service_name);
        world.add_service(service_name, true);
    })
}

fn should_be_healthy(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🏗️ Infrastructure: Verifying healthy status");
        assert!(world.services.get("vault").unwrap_or(&false), "Vault service should be healthy");
    })
}

fn all_services_operational(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🏗️ Infrastructure: Verifying all services operational");
        let healthy_count = world.services.values().filter(|&&healthy| healthy).count();
        assert!(healthy_count >= 1, "At least one service should be operational");
        println!("✅ {} services are operational", healthy_count);
    })
}

// =============================================================================
// AUTHENTICATION TEAM - User Management & Security
// =============================================================================

/// Authentication team owns user lifecycle and security steps
pub struct AuthenticationSteps;

impl StepBuilder<EnterpriseWorld> for AuthenticationSteps {
    fn register_steps(collection: Collection<EnterpriseWorld>) -> Collection<EnterpriseWorld> {
        collection
            .given(None, Regex::new(r"(\w+) is an admin user").unwrap(), user_is_admin)
            .given(None, Regex::new(r"(\w+) is a regular user").unwrap(), user_is_regular)
            .given(None, Regex::new(r#"(\w+) has role "([^"]+)""#).unwrap(), user_has_role)
            .when(None, Regex::new(r"(\w+) logs in with credentials").unwrap(), user_logs_in)
            .when(None, Regex::new(r"(\w+) attempts unauthorized access").unwrap(), unauthorized_access)
            .then(None, Regex::new(r"(\w+) should be authenticated").unwrap(), should_be_authenticated)
            .then(None, Regex::new(r"(\w+) should be denied access").unwrap(), should_be_denied)
    }
    
    fn domain_name() -> &'static str {
        "Authentication & Authorization"
    }
}

fn user_is_admin(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        println!("👤 Auth: Creating admin user: {}", user);
        world.authenticate_user(user, "admin");
    })
}

fn user_is_regular(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        println!("👤 Auth: Creating regular user: {}", user);
        world.authenticate_user(user, "user");
    })
}

fn user_has_role(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        let role = &ctx.matches[2].1;
        println!("👤 Auth: Assigning role '{}' to user: {}", role, user);
        world.authenticate_user(user, role);
    })
}

fn user_logs_in(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        println!("👤 Auth: User '{}' logging in", user);
        world.log_audit_event(json!({
            "action": "user_login",
            "user": user,
            "result": "success",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn unauthorized_access(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        println!("👤 Auth: User '{}' attempting unauthorized access", user);
        world.log_audit_event(json!({
            "action": "unauthorized_access",
            "user": user,
            "result": "denied",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn should_be_authenticated(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        println!("👤 Auth: Verifying user '{}' is authenticated", user);
        assert!(world.users.contains_key(user), "User {} should be authenticated", user);
    })
}

fn should_be_denied(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        println!("👤 Auth: Verifying user '{}' access was denied", user);
        // Check audit log for denial
        let denied = world.audit_events.iter().any(|event| {
            event.get("user").and_then(|u| u.as_str()) == Some(user) &&
            event.get("result").and_then(|r| r.as_str()) == Some("denied")
        });
        assert!(denied, "User {} should have been denied access", user);
    })
}

// =============================================================================
// CRYPTOGRAPHY TEAM - Key Management & Encryption
// =============================================================================

/// Cryptography team owns key lifecycle and encryption steps
pub struct CryptographySteps;

impl StepBuilder<EnterpriseWorld> for CryptographySteps {
    fn register_steps(collection: Collection<EnterpriseWorld>) -> Collection<EnterpriseWorld> {
        collection
            .given(None, Regex::new(r"crypto service is available").unwrap(), crypto_service_available)
            .when(None, Regex::new(r#"(\w+) creates a key "([^"]+)""#).unwrap(), create_key)
            .when(None, Regex::new(r#"(\w+) creates a "([^"]+)" key "([^"]+)""#).unwrap(), create_typed_key)
            .when(None, Regex::new(r#"encrypting data with key "([^"]+)""#).unwrap(), encrypt_data)
            .when(None, Regex::new(r#"rotating key "([^"]+)""#).unwrap(), rotate_key)
            .then(None, Regex::new(r"the key should be created successfully").unwrap(), key_created_successfully)
            .then(None, Regex::new(r#"key "([^"]+)" should be available"#).unwrap(), key_should_be_available)
            .then(None, Regex::new(r"the encryption should succeed").unwrap(), encryption_should_succeed)
    }
    
    fn domain_name() -> &'static str {
        "Cryptographic Operations"
    }
}

fn crypto_service_available(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🔐 Crypto: Crypto service is available");
        world.add_service("crypto", true);
    })
}

fn create_key(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        let key_id = &ctx.matches[2].1;
        println!("🔐 Crypto: User '{}' creating key '{}'", user, key_id);
        world.create_key(key_id, "AES-256");
        world.log_audit_event(json!({
            "action": "key_creation",
            "user": user,
            "key_id": key_id,
            "result": "success",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn create_typed_key(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let user = &ctx.matches[1].1;
        let key_type = &ctx.matches[2].1;
        let key_id = &ctx.matches[3].1;
        println!("🔐 Crypto: User '{}' creating {} key '{}'", user, key_type, key_id);
        world.create_key(key_id, key_type);
        world.log_audit_event(json!({
            "action": "typed_key_creation",
            "user": user,
            "key_type": key_type,
            "key_id": key_id,
            "result": "success",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn encrypt_data(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let key_id = &ctx.matches[1].1;
        println!("🔐 Crypto: Encrypting data with key '{}'", key_id);
        assert!(world.keys.contains_key(key_id), "Key {} should exist", key_id);
        world.log_audit_event(json!({
            "action": "data_encryption",
            "key_id": key_id,
            "result": "success",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn rotate_key(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let key_id = &ctx.matches[1].1;
        println!("🔐 Crypto: Rotating key '{}'", key_id);
        world.log_audit_event(json!({
            "action": "key_rotation",
            "key_id": key_id,
            "result": "success",
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn key_created_successfully(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🔐 Crypto: Verifying key creation");
        assert!(!world.keys.is_empty(), "At least one key should be created");
    })
}

fn key_should_be_available(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let key_id = &ctx.matches[1].1;
        println!("🔐 Crypto: Verifying key '{}' is available", key_id);
        assert!(world.keys.contains_key(key_id), "Key {} should be available", key_id);
    })
}

fn encryption_should_succeed(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("🔐 Crypto: Verifying encryption success");
        let encryption_events = world.audit_events.iter().filter(|event| {
            event.get("action").and_then(|a| a.as_str()) == Some("data_encryption")
        }).count();
        assert!(encryption_events > 0, "At least one encryption operation should have occurred");
    })
}

// =============================================================================
// COMPLIANCE TEAM - Audit & Licensing
// =============================================================================

// Use macro to demonstrate different implementation approaches
step_builder!(
    ComplianceSteps,
    "Audit & Compliance",
    EnterpriseWorld,
    |collection| {
        collection
            .given(None, Regex::new(r#"audit logging is enabled"#).unwrap(), audit_logging_enabled)
            .given(None, Regex::new(r#"license allows "([^"]+)" features"#).unwrap(), license_allows_features)
            .when(None, Regex::new(r"generating compliance report").unwrap(), generate_compliance_report)
            .when(None, Regex::new(r#"checking feature "([^"]+)" availability"#).unwrap(), check_feature_availability)
            .then(None, Regex::new(r"audit events should be recorded").unwrap(), audit_events_recorded)
            .then(None, Regex::new(r#"feature "([^"]+)" should be available"#).unwrap(), feature_should_be_available)
            .then(None, Regex::new(r"compliance requirements should be met").unwrap(), compliance_requirements_met)
    }
);

fn audit_logging_enabled(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("📋 Compliance: Audit logging is enabled");
        world.add_service("audit", true);
    })
}

fn license_allows_features(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let features = &ctx.matches[1].1;
        println!("📋 Compliance: License allows '{}' features", features);
        let feature_list: Vec<&str> = features.split(',').map(|s| s.trim()).collect();
        world.set_license("enterprise", feature_list);
    })
}

fn generate_compliance_report(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("📋 Compliance: Generating compliance report");
        world.log_audit_event(json!({
            "action": "compliance_report_generation",
            "events_count": world.audit_events.len(),
            "services_count": world.services.len(),
            "users_count": world.users.len(),
            "keys_count": world.keys.len(),
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn check_feature_availability(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let feature = &ctx.matches[1].1;
        println!("📋 Compliance: Checking feature '{}' availability", feature);
        world.log_audit_event(json!({
            "action": "feature_availability_check",
            "feature": feature,
            "timestamp": "2025-01-26T10:00:00Z"
        }));
    })
}

fn audit_events_recorded(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("📋 Compliance: Verifying audit events are recorded");
        assert!(!world.audit_events.is_empty(), "Audit events should be recorded");
        println!("✅ {} audit events recorded", world.audit_events.len());
    })
}

fn feature_should_be_available(world: &mut EnterpriseWorld, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let feature = &ctx.matches[1].1;
        println!("📋 Compliance: Verifying feature '{}' is available", feature);
        if let Some(license) = &world.license_info {
            if let Some(features) = license.get("features") {
                if let Some(features_array) = features.as_array() {
                    let feature_available = features_array.iter().any(|f| {
                        f.as_str().map_or(false, |s| s == feature)
                    });
                    assert!(feature_available, "Feature '{}' should be available in license", feature);
                }
            }
        }
    })
}

fn compliance_requirements_met(world: &mut EnterpriseWorld, _ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        println!("📋 Compliance: Verifying compliance requirements");
        
        // Check audit coverage
        let has_user_events = world.audit_events.iter().any(|e| {
            e.get("action").and_then(|a| a.as_str()).unwrap_or("").contains("user")
        });
        
        let has_key_events = world.audit_events.iter().any(|e| {
            e.get("action").and_then(|a| a.as_str()).unwrap_or("").contains("key")
        });
        
        assert!(has_user_events, "User audit events should be present for compliance");
        assert!(has_key_events, "Key audit events should be present for compliance");
        
        println!("✅ Compliance requirements met: {} audit events across {} domains", 
                world.audit_events.len(),
                world.services.len());
    })
}

// =============================================================================
// MAIN: ENTERPRISE BDD ARCHITECTURE DEMONSTRATION
// =============================================================================

#[tokio::main]
async fn main() {
    println!("🚀 Enterprise Modular BDD Architecture Example");
    println!("==============================================");
    
    // Method 1: Using individual step builders (team-owned approach)
    println!("\n📋 Building step collection using team-owned builders...");
    
    let infrastructure_steps = InfrastructureSteps::register_steps(Collection::new());
    let auth_steps = AuthenticationSteps::register_steps(Collection::new());
    let crypto_steps = CryptographySteps::register_steps(Collection::new());
    let compliance_steps = ComplianceSteps::register_steps(Collection::new());
    
    println!("✅ Infrastructure Team: {} steps registered", 
            infrastructure_steps.total_len());
    println!("✅ Authentication Team: {} steps registered", 
            auth_steps.total_len());
    println!("✅ Cryptography Team: {} steps registered", 
            crypto_steps.total_len());
    println!("✅ Compliance Team: {} steps registered", 
            compliance_steps.total_len());
    
    // Method 2: Using Collection::compose for enterprise-scale composition
    println!("\n🏗️ Composing enterprise step collection...");
    
    let enterprise_collection = Collection::compose(vec![
        infrastructure_steps,
        auth_steps,
        crypto_steps,
        compliance_steps,
    ]);
    
    let total_steps = enterprise_collection.total_len();
    
    println!("✅ Enterprise Collection: {} total steps across 4 domains", total_steps);
    println!("   - Given: {} steps", enterprise_collection.given_len());
    println!("   - When:  {} steps", enterprise_collection.when_len());
    println!("   - Then:  {} steps", enterprise_collection.then_len());
    
    // Method 3: Using compose_step_builders for functional composition
    println!("\n🔧 Alternative: Functional composition approach...");
    
    let builders: Vec<Box<dyn Fn(Collection<EnterpriseWorld>) -> Collection<EnterpriseWorld>>> = vec![
        Box::new(InfrastructureSteps::register_steps),
        Box::new(AuthenticationSteps::register_steps), 
        Box::new(CryptographySteps::register_steps),
        Box::new(ComplianceSteps::register_steps),
    ];
    
    let functional_collection = compose_step_builders(builders);
    let functional_total = functional_collection.total_len();
    
    println!("✅ Functional Collection: {} total steps", functional_total);
    
    // Demonstrate step execution with mock world
    println!("\n🧪 Simulating step execution...");
    let mut world = EnterpriseWorld::default();
    
    // Simulate some operations
    world.add_service("vault", true);
    world.authenticate_user("Alice", "admin");
    world.create_key("test-key-1", "RSA-2048");
    world.log_audit_event(json!({
        "action": "demonstration",
        "message": "Enterprise BDD architecture working",
        "timestamp": "2025-01-26T10:00:00Z"
    }));
    
    println!("✅ World State:");
    println!("   - Services: {}", world.services.len());
    println!("   - Users: {}", world.users.len());
    println!("   - Keys: {}", world.keys.len());
    println!("   - Audit Events: {}", world.audit_events.len());
    
    println!("\n🎯 Enterprise BDD Benefits Demonstrated:");
    println!("   ✅ Team Ownership: 4 domain-specific step builders");
    println!("   ✅ Scalability: {} steps without conflicts", total_steps);
    println!("   ✅ Maintainability: Clean domain separation");
    println!("   ✅ Reusability: Step builders can be mixed and matched");
    println!("   ✅ Testability: Each domain can be unit tested independently");
    
    println!("\n🏆 Ready for production enterprise BDD testing!");
}