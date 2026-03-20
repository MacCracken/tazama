//! Hardware accelerator detection and caching via ai-hwaccel.

use ai_hwaccel::{AcceleratorRegistry, AcceleratorType, CachedRegistry};
use std::sync::LazyLock;
use std::time::Duration;

/// Cached hardware registry with 5-minute TTL.
/// First access triggers detection; subsequent calls return cached results.
static REGISTRY: LazyLock<CachedRegistry> =
    LazyLock::new(|| CachedRegistry::new(Duration::from_secs(300)));

/// Get the cached accelerator registry.
pub fn registry() -> std::sync::Arc<AcceleratorRegistry> {
    REGISTRY.get()
}

/// Check if any GPU accelerator is available (CUDA, ROCm, Vulkan, etc.)
pub fn has_gpu() -> bool {
    registry().has_accelerator()
}

/// Check if VAAPI hardware encoding is likely available (AMD/Intel GPU detected).
pub fn has_vaapi() -> bool {
    let reg = registry();
    reg.available().iter().any(|p| {
        matches!(
            p.accelerator,
            AcceleratorType::RocmGpu { .. }
                | AcceleratorType::VulkanGpu { .. }
                | AcceleratorType::IntelOneApi { .. }
        )
    })
}

/// Check if NVENC hardware encoding is likely available (NVIDIA GPU detected).
pub fn has_nvenc() -> bool {
    let reg = registry();
    reg.available()
        .iter()
        .any(|p| matches!(p.accelerator, AcceleratorType::CudaGpu { .. }))
}

/// Get a summary of available accelerators for display/logging.
pub fn hardware_summary() -> Vec<HardwareInfo> {
    let reg = registry();
    reg.available()
        .iter()
        .map(|p| HardwareInfo {
            name: format!("{:?}", p.accelerator),
            family: format!("{}", p.accelerator.family()),
            memory_bytes: p.memory_bytes,
            available: p.available,
            memory_used_bytes: p.memory_used_bytes,
            memory_free_bytes: p.memory_free_bytes,
            temperature_c: p.temperature_c,
            gpu_utilization_percent: p.gpu_utilization_percent,
        })
        .collect()
}

/// Hardware info for IPC/display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardwareInfo {
    pub name: String,
    pub family: String,
    pub memory_bytes: u64,
    pub available: bool,
    pub memory_used_bytes: Option<u64>,
    pub memory_free_bytes: Option<u64>,
    pub temperature_c: Option<u32>,
    pub gpu_utilization_percent: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_at_least_cpu() {
        let reg = registry();
        assert!(
            !reg.available().is_empty(),
            "registry should detect at least one accelerator (CPU)"
        );
    }

    #[test]
    fn has_gpu_returns_bool() {
        // Just ensure it doesn't panic
        let _ = has_gpu();
    }

    #[test]
    fn hardware_summary_not_empty() {
        let summary = hardware_summary();
        assert!(
            !summary.is_empty(),
            "hardware summary should contain at least CPU"
        );
    }

    #[test]
    fn hardware_info_serde_roundtrip() {
        let info = HardwareInfo {
            name: "TestGpu".to_string(),
            family: "cuda".to_string(),
            memory_bytes: 8_000_000_000,
            available: true,
            memory_used_bytes: Some(2_000_000_000),
            memory_free_bytes: Some(6_000_000_000),
            temperature_c: Some(65),
            gpu_utilization_percent: Some(42),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: HardwareInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "TestGpu");
        assert_eq!(back.family, "cuda");
        assert_eq!(back.memory_bytes, 8_000_000_000);
        assert!(back.available);
        assert_eq!(back.memory_used_bytes, Some(2_000_000_000));
        assert_eq!(back.memory_free_bytes, Some(6_000_000_000));
        assert_eq!(back.temperature_c, Some(65));
        assert_eq!(back.gpu_utilization_percent, Some(42));
    }

    #[test]
    fn hardware_info_serde_with_none_fields() {
        let info = HardwareInfo {
            name: "CpuOnly".to_string(),
            family: "cpu".to_string(),
            memory_bytes: 16_000_000_000,
            available: true,
            memory_used_bytes: None,
            memory_free_bytes: None,
            temperature_c: None,
            gpu_utilization_percent: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: HardwareInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "CpuOnly");
        assert!(back.memory_used_bytes.is_none());
        assert!(back.temperature_c.is_none());
    }
}
