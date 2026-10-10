//! Establish a WebGPU device and preserve its reported hardware identity.

use bevy_physics_bench::BenchError;
use khal::{
    backend::{GpuBackend, WebGpu},
    re_exports::wgpu,
};

/// Request a device from the same adapter whose identity is stored with
/// benchmark results.
pub(crate) async fn create() -> Result<(GpuBackend, serde_json::Value), BenchError> {
    // Use the published Khal 0.4 WebGPU boundary without enabling unchecked
    // shader passthrough.
    // https://docs.rs/khal/0.4.0/khal/backend/webgpu/struct.WebGpu.html
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(failure)?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(failure)?;
    let backend_name = info.backend;
    let metadata = adapter_metadata(&info.name, &format!("{backend_name:?}"), &info.driver_info);
    Ok((
        GpuBackend::WebGpu(WebGpu::from_device(instance, adapter, device, queue)),
        metadata,
    ))
}

/// Preserve undisclosed browser adapter fields without inventing hardware facts.
fn adapter_metadata(name: &str, backend: &str, driver: &str) -> serde_json::Value {
    // Browser privacy settings may leave the adapter name and driver string empty.
    let name = if name.is_empty() {
        "Undisclosed GPU"
    } else {
        name
    };
    let driver = if driver.is_empty() {
        None
    } else {
        Some(driver)
    };
    serde_json::json!({ "name": name, "backend": backend, "driver": driver })
}

/// Keep the original graphics error available through the shared
/// integration error's source chain.
pub(crate) fn failure(error: impl std::error::Error + Send + Sync + 'static) -> BenchError {
    BenchError::Engine(Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_preserves_reported_values_and_marks_undisclosed_fields() {
        assert_eq!(
            adapter_metadata("RTX", "Vulkan", "595"),
            serde_json::json!({"name": "RTX", "backend": "Vulkan", "driver": "595"})
        );
        assert_eq!(
            adapter_metadata("", "BrowserWebGpu", ""),
            serde_json::json!({"name": "Undisclosed GPU", "backend": "BrowserWebGpu", "driver": null})
        );
    }
}
