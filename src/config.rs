use std::env;

#[allow(dead_code)]
const HEALTHY_THRESHOLD_NAME: &str = "XOLOTL_HEALTHY_THRESHOLD_MS";
#[allow(dead_code)]
const STALE_THRESHOLD_NAME: &str = "XOLOTL_STALE_THRESHOLD_MS";
#[allow(dead_code)]
const CLEANUP_INTERVAL_NAME: &str = "XOLOTL_CLEANUP_INTERVAL_SECS";
#[allow(dead_code)]
const HEALTHY_THRESHOLD_MS: &str = "60000";
#[allow(dead_code)]
const STALE_THRESHOLD_MS: &str = "90000";
#[allow(dead_code)]
const CLEANUP_INTERVAL_SECS: &str = "30";

#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub healthy_threshold_ms: u64,
    pub stale_threshold_ms: u64,
    pub cleanup_interval_secs: u64,
}

impl HealthConfig {
    pub fn from_env() -> Self {
        Self {
            healthy_threshold_ms: env::var(HEALTHY_THRESHOLD_NAME)
                .unwrap_or_else(|_| HEALTHY_THRESHOLD_MS.to_string())
                .parse()
                .expect("XOLOTL_HEALTHY_THRESHOLD_MS must be a valid positive number"),
            stale_threshold_ms: env::var(STALE_THRESHOLD_NAME)
                .unwrap_or_else(|_| STALE_THRESHOLD_MS.to_string())
                .parse()
                .expect("XOLOTL_STALE_THRESHOLD_MS must be a valid positive number"),
            cleanup_interval_secs: env::var(CLEANUP_INTERVAL_NAME)
                .unwrap_or_else(|_| CLEANUP_INTERVAL_SECS.to_string())
                .parse()
                .expect("XOLOTL_CLEANUP_INTERVAL_SECS must be a positive number"),
        }
    }
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            healthy_threshold_ms: HEALTHY_THRESHOLD_MS.parse::<u64>().unwrap(),
            stale_threshold_ms: STALE_THRESHOLD_MS.parse::<u64>().unwrap(),
            cleanup_interval_secs: CLEANUP_INTERVAL_SECS.parse::<u64>().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    fn clear_test_env_vars() {
        unsafe {
            env::remove_var(HEALTHY_THRESHOLD_NAME);
            env::remove_var(STALE_THRESHOLD_NAME);
            env::remove_var(CLEANUP_INTERVAL_NAME);
        }
    }

    #[test]
    #[serial]
    fn test_default_values() {
        clear_test_env_vars();

        let config = HealthConfig::from_env();

        assert_eq!(
            config.healthy_threshold_ms,
            HEALTHY_THRESHOLD_MS.parse::<u64>().unwrap()
        );
        assert_eq!(
            config.stale_threshold_ms,
            STALE_THRESHOLD_MS.parse::<u64>().unwrap()
        );
        assert_eq!(
            config.cleanup_interval_secs,
            CLEANUP_INTERVAL_SECS.parse::<u64>().unwrap()
        );
    }

    #[test]
    #[serial]
    fn test_custom_env_values() {
        clear_test_env_vars();

        unsafe {
            env::set_var(HEALTHY_THRESHOLD_NAME, "30000");
            env::set_var(STALE_THRESHOLD_NAME, "45000");
            env::set_var(CLEANUP_INTERVAL_NAME, "60");
        }

        let config = HealthConfig::from_env();

        assert_eq!(config.healthy_threshold_ms, 30000);
        assert_eq!(config.stale_threshold_ms, 45000);
        assert_eq!(config.cleanup_interval_secs, 60);

        clear_test_env_vars();
    }

    #[test]
    #[serial]
    fn test_partial_env_values() {
        clear_test_env_vars();

        // Only set one env var, others should use defaults
        unsafe {
            env::set_var(HEALTHY_THRESHOLD_NAME, "45000");
        }

        let config = HealthConfig::from_env();

        assert_eq!(config.healthy_threshold_ms, 45000);
        assert_eq!(config.stale_threshold_ms, 90000); // default
        assert_eq!(config.cleanup_interval_secs, 30); // default

        clear_test_env_vars();
    }

    #[test]
    #[serial]
    #[should_panic(expected = "XOLOTL_HEALTHY_THRESHOLD_MS must be a valid positive number")]
    fn test_invalid_healthy_threshold() {
        clear_test_env_vars();

        unsafe {
            env::set_var(HEALTHY_THRESHOLD_NAME, "not_a_number");
        }

        HealthConfig::from_env();
    }

    #[test]
    #[serial]
    #[should_panic(expected = "XOLOTL_STALE_THRESHOLD_MS must be a valid positive number")]
    fn test_invalid_stale_threshold() {
        clear_test_env_vars();

        unsafe {
            env::set_var(STALE_THRESHOLD_NAME, "1000.5");
        }

        HealthConfig::from_env();
    }

    #[test]
    #[serial]
    #[should_panic(expected = "XOLOTL_CLEANUP_INTERVAL_SECS must be a positive number")]
    fn test_invalid_cleanup_interval() {
        clear_test_env_vars();

        unsafe {
            env::set_var(CLEANUP_INTERVAL_NAME, "-5");
        }

        HealthConfig::from_env();
    }
}
