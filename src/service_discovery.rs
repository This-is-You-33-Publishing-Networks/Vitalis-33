//! Service Discovery — v393
//! Service registration, deregistration, health checking, and discovery.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub name: String,
    pub instance_id: String,
    pub host: String,
    pub port: u16,
    pub health: HealthStatus,
    pub metadata: HashMap<String, String>,
    pub heartbeat_count: u64,
    pub missed_heartbeats: u32,
    pub max_missed: u32,
}

impl ServiceInstance {
    pub fn new(name: &str, instance_id: &str, host: &str, port: u16) -> Self {
        Self {
            name: name.to_string(),
            instance_id: instance_id.to_string(),
            host: host.to_string(),
            port,
            health: HealthStatus::Unknown,
            metadata: HashMap::new(),
            heartbeat_count: 0,
            missed_heartbeats: 0,
            max_missed: 3,
        }
    }

    pub fn heartbeat(&mut self) {
        self.heartbeat_count += 1;
        self.missed_heartbeats = 0;
        self.health = HealthStatus::Healthy;
    }

    pub fn miss_heartbeat(&mut self) {
        self.missed_heartbeats += 1;
        if self.missed_heartbeats >= self.max_missed {
            self.health = HealthStatus::Unhealthy;
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.health == HealthStatus::Healthy
    }
}

/// Service registry for discovery.
#[derive(Debug)]
pub struct ServiceRegistry {
    services: HashMap<String, Vec<ServiceInstance>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self { services: HashMap::new() }
    }

    pub fn register(&mut self, instance: ServiceInstance) {
        let name = instance.name.clone();
        self.services.entry(name).or_default().push(instance);
    }

    pub fn deregister(&mut self, name: &str, instance_id: &str) -> bool {
        let mut found = false;
        if let Some(instances) = self.services.get_mut(name) {
            let len_before = instances.len();
            instances.retain(|i| i.instance_id != instance_id);
            found = instances.len() < len_before;
        }
        // Remove empty entry after releasing mutable borrow.
        if self.services.get(name).map(|v| v.is_empty()).unwrap_or(false) {
            self.services.remove(name);
        }
        found
    }

    /// Discover healthy instances of a service.
    pub fn discover(&self, name: &str) -> Vec<&ServiceInstance> {
        if let Some(instances) = self.services.get(name) {
            instances.iter().filter(|i| i.is_healthy()).collect()
        } else {
            Vec::new()
        }
    }

    /// Discover all instances (regardless of health).
    pub fn discover_all(&self, name: &str) -> Vec<&ServiceInstance> {
        self.services.get(name).map(|v| v.iter().collect()).unwrap_or_default()
    }

    pub fn health_check(&self, name: &str) -> Vec<(String, bool)> {
        if let Some(instances) = self.services.get(name) {
            instances.iter()
                .map(|i| (i.instance_id.clone(), i.is_healthy()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn heartbeat(&mut self, name: &str, instance_id: &str) -> bool {
        if let Some(instances) = self.services.get_mut(name) {
            if let Some(inst) = instances.iter_mut().find(|i| i.instance_id == instance_id) {
                inst.heartbeat();
                return true;
            }
        }
        false
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    pub fn instance_count(&self, name: &str) -> usize {
        self.services.get(name).map(|v| v.len()).unwrap_or(0)
    }

    pub fn list_services(&self) -> Vec<&str> {
        self.services.keys().map(|s| s.as_str()).collect()
    }

    pub fn is_healthy(&self, name: &str, instance_id: &str) -> bool {
        if let Some(instances) = self.services.get(name) {
            instances.iter()
                .find(|i| i.instance_id == instance_id)
                .map(|i| i.is_healthy())
                .unwrap_or(false)
        } else {
            false
        }
    }
}

static SD_REGISTRY: LazyLock<Mutex<ServiceRegistry>> =
    LazyLock::new(|| Mutex::new(ServiceRegistry::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_register(service_id: i64, port: i64) -> i64 {
    let name = format!("service_{}", service_id);
    let instance_id = format!("inst_{}_{}", service_id, port);
    let mut instance = ServiceInstance::new(&name, &instance_id, "127.0.0.1", port as u16);
    instance.heartbeat(); // Mark healthy on registration. 
    let mut registry = SD_REGISTRY.lock().unwrap();
    registry.register(instance);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_deregister(service_id: i64, port: i64) -> i64 {
    let name = format!("service_{}", service_id);
    let instance_id = format!("inst_{}_{}", service_id, port);
    let mut registry = SD_REGISTRY.lock().unwrap();
    if registry.deregister(&name, &instance_id) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_discover(service_id: i64) -> i64 {
    let name = format!("service_{}", service_id);
    let registry = SD_REGISTRY.lock().unwrap();
    registry.discover(&name).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_health_check(service_id: i64) -> i64 {
    let name = format!("service_{}", service_id);
    let registry = SD_REGISTRY.lock().unwrap();
    let checks = registry.health_check(&name);
    checks.iter().filter(|(_, h)| *h).count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_service_count() -> i64 {
    let registry = SD_REGISTRY.lock().unwrap();
    registry.service_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_is_healthy(service_id: i64, port: i64) -> i64 {
    let name = format!("service_{}", service_id);
    let instance_id = format!("inst_{}_{}", service_id, port);
    let registry = SD_REGISTRY.lock().unwrap();
    if registry.is_healthy(&name, &instance_id) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_list_services() -> i64 {
    let registry = SD_REGISTRY.lock().unwrap();
    registry.service_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_sd_heartbeat(service_id: i64, port: i64) -> i64 {
    let name = format!("service_{}", service_id);
    let instance_id = format!("inst_{}_{}", service_id, port);
    let mut registry = SD_REGISTRY.lock().unwrap();
    if registry.heartbeat(&name, &instance_id) { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_instance_new() {
        let inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        assert_eq!(inst.name, "api");
        assert_eq!(inst.health, HealthStatus::Unknown);
    }

    #[test]
    fn test_heartbeat() {
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.heartbeat();
        assert!(inst.is_healthy());
        assert_eq!(inst.heartbeat_count, 1);
    }

    #[test]
    fn test_miss_heartbeat() {
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.heartbeat();
        inst.miss_heartbeat();
        inst.miss_heartbeat();
        inst.miss_heartbeat();
        assert!(!inst.is_healthy());
    }

    #[test]
    fn test_heartbeat_resets_missed() {
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.miss_heartbeat();
        inst.miss_heartbeat();
        inst.heartbeat(); // resets
        assert!(inst.is_healthy());
        assert_eq!(inst.missed_heartbeats, 0);
    }

    #[test]
    fn test_registry_new() {
        let reg = ServiceRegistry::new();
        assert_eq!(reg.service_count(), 0);
    }

    #[test]
    fn test_register() {
        let mut reg = ServiceRegistry::new();
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.heartbeat();
        reg.register(inst);
        assert_eq!(reg.service_count(), 1);
    }

    #[test]
    fn test_discover_healthy() {
        let mut reg = ServiceRegistry::new();
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.heartbeat();
        reg.register(inst);
        let found = reg.discover("api");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_discover_unhealthy() {
        let mut reg = ServiceRegistry::new();
        let inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        // Not healthy (unknown status).
        reg.register(inst);
        let found = reg.discover("api");
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_discover_all() {
        let mut reg = ServiceRegistry::new();
        let inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        reg.register(inst);
        let found = reg.discover_all("api");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_deregister() {
        let mut reg = ServiceRegistry::new();
        let inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        reg.register(inst);
        assert!(reg.deregister("api", "inst1"));
        assert_eq!(reg.service_count(), 0);
    }

    #[test]
    fn test_deregister_nonexistent() {
        let mut reg = ServiceRegistry::new();
        assert!(!reg.deregister("api", "inst1"));
    }

    #[test]
    fn test_multiple_instances() {
        let mut reg = ServiceRegistry::new();
        let mut i1 = ServiceInstance::new("api", "inst1", "host1", 8080);
        i1.heartbeat();
        let mut i2 = ServiceInstance::new("api", "inst2", "host2", 8081);
        i2.heartbeat();
        reg.register(i1);
        reg.register(i2);
        assert_eq!(reg.instance_count("api"), 2);
        assert_eq!(reg.discover("api").len(), 2);
    }

    #[test]
    fn test_multiple_services() {
        let mut reg = ServiceRegistry::new();
        let inst1 = ServiceInstance::new("api", "a1", "host1", 8080);
        let inst2 = ServiceInstance::new("web", "w1", "host2", 80);
        reg.register(inst1);
        reg.register(inst2);
        assert_eq!(reg.service_count(), 2);
    }

    #[test]
    fn test_list_services() {
        let mut reg = ServiceRegistry::new();
        let inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        reg.register(inst);
        let services = reg.list_services();
        assert!(services.contains(&"api"));
    }

    #[test]
    fn test_health_check() {
        let mut reg = ServiceRegistry::new();
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.heartbeat();
        reg.register(inst);
        let checks = reg.health_check("api");
        assert_eq!(checks.len(), 1);
        assert!(checks[0].1);
    }

    #[test]
    fn test_heartbeat_via_registry() {
        let mut reg = ServiceRegistry::new();
        let inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        reg.register(inst);
        assert!(reg.heartbeat("api", "inst1"));
        assert!(reg.is_healthy("api", "inst1"));
    }

    #[test]
    fn test_heartbeat_unknown_service() {
        let mut reg = ServiceRegistry::new();
        assert!(!reg.heartbeat("api", "inst1"));
    }

    #[test]
    fn test_is_healthy() {
        let mut reg = ServiceRegistry::new();
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.heartbeat();
        reg.register(inst);
        assert!(reg.is_healthy("api", "inst1"));
    }

    #[test]
    fn test_discover_missing_service() {
        let reg = ServiceRegistry::new();
        assert!(reg.discover("nonexistent").is_empty());
    }

    #[test]
    fn test_metadata() {
        let mut inst = ServiceInstance::new("api", "inst1", "localhost", 8080);
        inst.metadata.insert("version".into(), "1.0".into());
        assert_eq!(inst.metadata.get("version").unwrap(), "1.0");
    }
}
