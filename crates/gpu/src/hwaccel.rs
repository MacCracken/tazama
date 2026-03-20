use ai_hwaccel::AcceleratorRegistry;

/// Hardware information for a detected GPU accelerator.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuHardwareInfo {
    pub name: String,
    pub memory_bytes: u64,
    pub compute_capability: Option<String>,
    pub driver_version: Option<String>,
}

/// Query available GPU hardware via ai-hwaccel.
pub fn detect_gpu_hardware() -> Vec<GpuHardwareInfo> {
    let registry = AcceleratorRegistry::detect();
    registry
        .available()
        .iter()
        .filter(|p| p.accelerator.is_gpu())
        .map(|p| GpuHardwareInfo {
            name: format!("{:?}", p.accelerator),
            memory_bytes: p.memory_bytes,
            compute_capability: p.compute_capability.clone(),
            driver_version: p.driver_version.clone(),
        })
        .collect()
}

/// Log detected GPU hardware from ai-hwaccel. Called during GPU context init.
pub(crate) fn log_detected_hardware() {
    let registry = AcceleratorRegistry::detect();
    if registry.has_accelerator() {
        for profile in registry.available() {
            if profile.accelerator.is_gpu() {
                tracing::info!(
                    "ai-hwaccel detected GPU: {:?} ({} MB VRAM)",
                    profile.accelerator,
                    profile.memory_bytes / (1024 * 1024)
                );
            }
        }
    } else {
        tracing::info!("ai-hwaccel: no GPU accelerator detected, using CPU fallback");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_gpu_hardware_returns_vec() {
        let gpus = detect_gpu_hardware();
        // Should not panic; result is a vec (possibly empty in CI).
        let _ = gpus.len();
    }

    #[test]
    fn gpu_hardware_info_serde_roundtrip() {
        let info = GpuHardwareInfo {
            name: "TestGPU".to_string(),
            memory_bytes: 8 * 1024 * 1024 * 1024,
            compute_capability: Some("8.6".to_string()),
            driver_version: Some("535.129.03".to_string()),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let back: GpuHardwareInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, info.name);
        assert_eq!(back.memory_bytes, info.memory_bytes);
        assert_eq!(back.compute_capability, info.compute_capability);
        assert_eq!(back.driver_version, info.driver_version);
    }
}
