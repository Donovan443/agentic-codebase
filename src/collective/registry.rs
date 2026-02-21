//! Collective registry client.
//!
//! Provides a registry client for querying and publishing collective
//! intelligence data. Works in offline mode when no network is available.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::delta::CollectiveDelta;
use super::patterns::UsagePattern;

const DEFAULT_CACHE_MAINTENANCE_SECS: u64 = 300;
const DEFAULT_SLA_MAX_REGISTRY_OPS_PER_MIN: u32 = 1200;
const DEFAULT_HEALTH_LEDGER_EMIT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy)]
enum AutonomicProfile {
    Desktop,
    Cloud,
    Aggressive,
}

impl AutonomicProfile {
    fn from_env(name: &str) -> Self {
        let raw = read_env_string(name).unwrap_or_else(|| "desktop".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "cloud" => Self::Cloud,
            "aggressive" => Self::Aggressive,
            _ => Self::Desktop,
        }
    }

    fn cache_maintenance_secs(self) -> u64 {
        match self {
            Self::Desktop => DEFAULT_CACHE_MAINTENANCE_SECS,
            Self::Cloud => 120,
            Self::Aggressive => 60,
        }
    }

    fn sla_max_registry_ops_per_min(self) -> u32 {
        match self {
            Self::Desktop => DEFAULT_SLA_MAX_REGISTRY_OPS_PER_MIN,
            Self::Cloud => 4000,
            Self::Aggressive => 6000,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Cloud => "cloud",
            Self::Aggressive => "aggressive",
        }
    }
}

/// Operating mode for the registry client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryMode {
    /// Fully online — sync with remote registry.
    Online,
    /// Offline — use local cache only, no network.
    Offline,
}

impl std::fmt::Display for RegistryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "online"),
            Self::Offline => write!(f, "offline"),
        }
    }
}

/// A cached entry with TTL.
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// The cached value.
    value: T,
    /// When this entry was inserted.
    inserted_at: Instant,
    /// Time-to-live for this entry.
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    /// Check if this entry has expired.
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// A TTL-based cache for collective data.
#[derive(Debug)]
pub struct CollectiveCache {
    /// Cached patterns indexed by query key.
    patterns: HashMap<String, CacheEntry<Vec<UsagePattern>>>,
    /// Default TTL for cache entries.
    default_ttl: Duration,
}

impl CollectiveCache {
    /// Create a new cache with a default TTL.
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            patterns: HashMap::new(),
            default_ttl,
        }
    }

    /// Get cached patterns for a query key, if not expired.
    pub fn get_patterns(&self, key: &str) -> Option<&[UsagePattern]> {
        self.patterns
            .get(key)
            .filter(|entry| !entry.is_expired())
            .map(|entry| entry.value.as_slice())
    }

    /// Store patterns in the cache.
    pub fn put_patterns(&mut self, key: String, patterns: Vec<UsagePattern>) {
        self.patterns.insert(
            key,
            CacheEntry {
                value: patterns,
                inserted_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Remove expired entries from the cache.
    pub fn evict_expired(&mut self) {
        self.patterns.retain(|_, entry| !entry.is_expired());
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.patterns.clear();
    }

    /// Get the number of (possibly expired) entries in the cache.
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

impl Default for CollectiveCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(300))
    }
}

/// Registry client for collective intelligence data.
///
/// In offline mode, all queries return empty results and publish operations
/// are silently dropped. In online mode (not yet implemented), the client
/// would communicate with a remote registry server.
#[derive(Debug)]
pub struct RegistryClient {
    /// Operating mode.
    mode: RegistryMode,
    /// Local cache.
    cache: CollectiveCache,
    /// Registry endpoint URL (used in online mode).
    endpoint: Option<String>,
    /// Last time periodic cache maintenance ran.
    last_cache_maintenance: Instant,
    /// Interval between periodic cache maintenance runs.
    cache_maintenance_interval: Duration,
    /// Current autonomic profile.
    profile: AutonomicProfile,
    /// SLA threshold for maintenance throttling.
    sla_max_registry_ops_per_min: u32,
    /// Start of current operation-rate window.
    ops_window_started: Instant,
    /// Number of operations in current window.
    ops_window_count: u32,
    /// Number of times maintenance has been throttled.
    cache_maintenance_throttle_count: u64,
    /// Last time a health-ledger snapshot was emitted.
    last_health_ledger_emit: Instant,
    /// Minimum interval between health-ledger snapshots.
    health_ledger_emit_interval: Duration,
}

impl RegistryClient {
    /// Create a new registry client in offline mode.
    pub fn offline() -> Self {
        let profile = AutonomicProfile::from_env("ACB_AUTONOMIC_PROFILE");
        let cache_maintenance_interval = Duration::from_secs(read_env_u64(
            "ACB_COLLECTIVE_CACHE_MAINTENANCE_SECS",
            profile.cache_maintenance_secs(),
        ));
        let health_ledger_emit_interval = Duration::from_secs(
            read_env_u64(
                "ACB_HEALTH_LEDGER_EMIT_SECS",
                DEFAULT_HEALTH_LEDGER_EMIT_SECS,
            )
            .max(5),
        );
        Self {
            mode: RegistryMode::Offline,
            cache: CollectiveCache::default(),
            endpoint: None,
            last_cache_maintenance: Instant::now(),
            cache_maintenance_interval,
            profile,
            sla_max_registry_ops_per_min: read_env_u32(
                "ACB_SLA_MAX_REGISTRY_OPS_PER_MIN",
                profile.sla_max_registry_ops_per_min(),
            )
            .max(1),
            ops_window_started: Instant::now(),
            ops_window_count: 0,
            cache_maintenance_throttle_count: 0,
            last_health_ledger_emit: Instant::now()
                .checked_sub(health_ledger_emit_interval)
                .unwrap_or_else(Instant::now),
            health_ledger_emit_interval,
        }
    }

    /// Create a new registry client in online mode (stub).
    ///
    /// Note: Online mode is not yet implemented. The client will still
    /// behave as offline but will store the endpoint for future use.
    pub fn online(endpoint: String) -> Self {
        let profile = AutonomicProfile::from_env("ACB_AUTONOMIC_PROFILE");
        let cache_maintenance_interval = Duration::from_secs(read_env_u64(
            "ACB_COLLECTIVE_CACHE_MAINTENANCE_SECS",
            profile.cache_maintenance_secs(),
        ));
        let health_ledger_emit_interval = Duration::from_secs(
            read_env_u64(
                "ACB_HEALTH_LEDGER_EMIT_SECS",
                DEFAULT_HEALTH_LEDGER_EMIT_SECS,
            )
            .max(5),
        );
        Self {
            mode: RegistryMode::Online,
            cache: CollectiveCache::default(),
            endpoint: Some(endpoint),
            last_cache_maintenance: Instant::now(),
            cache_maintenance_interval,
            profile,
            sla_max_registry_ops_per_min: read_env_u32(
                "ACB_SLA_MAX_REGISTRY_OPS_PER_MIN",
                profile.sla_max_registry_ops_per_min(),
            )
            .max(1),
            ops_window_started: Instant::now(),
            ops_window_count: 0,
            cache_maintenance_throttle_count: 0,
            last_health_ledger_emit: Instant::now()
                .checked_sub(health_ledger_emit_interval)
                .unwrap_or_else(Instant::now),
            health_ledger_emit_interval,
        }
    }

    /// Get the current operating mode.
    pub fn mode(&self) -> &RegistryMode {
        &self.mode
    }

    /// Get the endpoint URL, if configured.
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    /// Query patterns from the registry.
    ///
    /// In offline mode, always returns an empty list.
    /// Checks cache first before making any (future) network calls.
    pub fn query_patterns(&mut self, language: &str, category: &str) -> Vec<UsagePattern> {
        self.record_operation();
        self.maybe_run_cache_maintenance();
        let cache_key = format!("{}:{}", language, category);

        // Check cache first.
        if let Some(cached) = self.cache.get_patterns(&cache_key) {
            return cached.to_vec();
        }

        match self.mode {
            RegistryMode::Offline => {
                tracing::debug!(
                    "Registry in offline mode; returning empty patterns for {}:{}.",
                    language,
                    category
                );
                Vec::new()
            }
            RegistryMode::Online => {
                // Online mode is a stub: log and return empty.
                tracing::debug!(
                    "Registry online query for {}:{} (not yet implemented).",
                    language,
                    category
                );
                Vec::new()
            }
        }
    }

    /// Publish a delta to the registry.
    ///
    /// In offline mode, the delta is silently dropped.
    /// Returns true if the delta was accepted (or dropped in offline mode).
    pub fn publish_delta(&mut self, _delta: &CollectiveDelta) -> bool {
        self.record_operation();
        self.maybe_run_cache_maintenance();
        match self.mode {
            RegistryMode::Offline => {
                tracing::debug!("Registry in offline mode; delta silently dropped.");
                true
            }
            RegistryMode::Online => {
                tracing::debug!("Registry publish (not yet implemented).");
                true
            }
        }
    }

    /// Access the internal cache.
    pub fn cache(&self) -> &CollectiveCache {
        &self.cache
    }

    /// Access the internal cache mutably.
    pub fn cache_mut(&mut self) -> &mut CollectiveCache {
        &mut self.cache
    }

    /// Run cache maintenance if the maintenance interval has elapsed.
    pub fn maybe_run_cache_maintenance(&mut self) {
        if self.last_cache_maintenance.elapsed() < self.cache_maintenance_interval {
            return;
        }
        if self.should_throttle_maintenance() {
            self.cache_maintenance_throttle_count =
                self.cache_maintenance_throttle_count.saturating_add(1);
            self.last_cache_maintenance = Instant::now();
            self.emit_health_ledger("throttled", 0);
            tracing::debug!(
                "collective cache maintenance throttled: ops_per_min={} threshold={}",
                self.registry_ops_per_min(),
                self.sla_max_registry_ops_per_min
            );
            return;
        }

        let before = self.cache.len();
        self.cache.evict_expired();
        let after = self.cache.len();
        self.last_cache_maintenance = Instant::now();
        let evicted = before.saturating_sub(after);
        self.emit_health_ledger("normal", evicted);

        if after < before {
            tracing::debug!(
                "collective cache maintenance evicted {} expired entries",
                evicted
            );
        }
    }

    fn record_operation(&mut self) {
        if self.ops_window_started.elapsed() >= Duration::from_secs(60) {
            self.ops_window_started = Instant::now();
            self.ops_window_count = 0;
        }
        self.ops_window_count = self.ops_window_count.saturating_add(1);
    }

    fn registry_ops_per_min(&self) -> u32 {
        let elapsed = self.ops_window_started.elapsed().as_secs().max(1);
        let scaled = (self.ops_window_count as u64)
            .saturating_mul(60)
            .saturating_div(elapsed);
        scaled.min(u32::MAX as u64) as u32
    }

    fn should_throttle_maintenance(&self) -> bool {
        self.registry_ops_per_min() > self.sla_max_registry_ops_per_min
    }

    fn emit_health_ledger(&mut self, maintenance_mode: &str, evicted: usize) {
        if self.last_health_ledger_emit.elapsed() < self.health_ledger_emit_interval {
            return;
        }

        let dir = resolve_health_ledger_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let path = dir.join("agentic-codebase.json");
        let tmp = dir.join("agentic-codebase.json.tmp");
        let payload = serde_json::json!({
            "project": "AgenticCodebase",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": "ok",
            "autonomic": {
                "profile": self.profile.as_str(),
                "mode": self.mode.to_string(),
                "maintenance_mode": maintenance_mode,
                "cache_maintenance_secs": self.cache_maintenance_interval.as_secs(),
                "throttle_count": self.cache_maintenance_throttle_count,
            },
            "sla": {
                "registry_ops_per_min": self.registry_ops_per_min(),
                "max_registry_ops_per_min": self.sla_max_registry_ops_per_min
            },
            "cache": {
                "entries": self.cache.len(),
                "evicted": evicted
            },
        });
        let Ok(bytes) = serde_json::to_vec_pretty(&payload) else {
            return;
        };
        if std::fs::write(&tmp, bytes).is_err() {
            return;
        }
        if std::fs::rename(&tmp, &path).is_err() {
            return;
        }
        self.last_health_ledger_emit = Instant::now();
    }
}

fn read_env_u64(name: &str, default_value: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_value)
}

fn read_env_u32(name: &str, default_value: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(default_value)
}

fn read_env_string(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string())
}

fn resolve_health_ledger_dir() -> PathBuf {
    if let Some(custom) = read_env_string("ACB_HEALTH_LEDGER_DIR") {
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }
    if let Some(custom) = read_env_string("AGENTRA_HEALTH_LEDGER_DIR") {
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }

    let home = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".agentra").join("health-ledger")
}
