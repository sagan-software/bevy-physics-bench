//! Establish a WebGPU device and preserve its reported hardware identity.

use bevy_physics_bench::{BenchError, BodyCount};
use khal::{
    backend::{GpuBackend, WebGpu},
    re_exports::wgpu,
};

/// Request a device from the same adapter whose identity is stored with
/// benchmark results.
pub(crate) async fn create(
    count: BodyCount,
) -> Result<(GpuBackend, serde_json::Value), BenchError> {
    // Use the published Khal 0.4 WebGPU boundary without enabling unchecked
    // shader passthrough.
    // https://docs.rs/khal/0.4.0/khal/backend/webgpu/struct.WebGpu.html
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(failure)?;
    let info = adapter.get_info();
    // Validate required storage before requesting a device or allocating scene buffers.
    let descriptor = wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: required_limits(count, &adapter.limits())?,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    };
    let (device, queue) = adapter.request_device(&descriptor).await.map_err(failure)?;
    let backend_name = info.backend;
    let metadata = adapter_metadata(&info.name, &format!("{backend_name:?}"), &info.driver_info);
    Ok((
        GpuBackend::WebGpu(WebGpu::from_device(instance, adapter, device, queue)),
        metadata,
    ))
}

/// Request the two buffer limits needed by the pinned primitive-scene profile.
fn required_limits(count: BodyCount, available: &wgpu::Limits) -> Result<wgpu::Limits, BenchError> {
    use nexus_rbd3d::{
        queries::GpuIndexedContact,
        shaders::dynamics::{TwoBodyConstraint, TwoBodyConstraintBuilder},
    };
    // Nexus 0.6.0 allocates two contact slots per pair; these are its largest records.
    // https://docs.rs/nexus_rbd3d/0.6.0/src/nexus_rbd3d/pipeline/rbd_state_from_rapier.rs.html
    let element_bytes = size_of::<GpuIndexedContact>()
        .max(size_of::<TwoBodyConstraint>())
        .max(size_of::<TwoBodyConstraintBuilder>());
    let element_bytes = u64::try_from(element_bytes).map_err(failure)?;
    // Bodies include one floor; pairs/body times contacts/pair times bytes/contact gives bytes.
    let bodies = u64::from(count.get()) + 1;
    let bytes = bodies * u64::from(crate::scene::PAIRS_PER_BODY) * 2 * element_bytes;
    let defaults = wgpu::Limits::default();
    let required = wgpu::Limits {
        max_buffer_size: defaults.max_buffer_size.max(bytes),
        max_storage_buffer_binding_size: defaults.max_storage_buffer_binding_size.max(bytes),
        ..defaults
    };
    // wgpu requires explicit limits and forbids requests beyond the selected adapter.
    // https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html
    if !required.check_limits(available) {
        return Err(BenchError::Unsupported(
            "scene exceeds this GPU's buffer limits; reduce body count",
        ));
    }
    Ok(required)
}

/// Preserve undisclosed browser adapter fields without inventing hardware facts.
fn adapter_metadata(name: &str, backend: &str, driver: &str) -> serde_json::Value {
    // Browser privacy settings may leave the adapter name and driver string empty.
    let name = if name.is_empty() {
        "Undisclosed GPU"
    } else {
        name
    };
    // Null identifies a withheld driver rather than an empty reported version.
    let driver = if driver.is_empty() {
        None
    } else {
        Some(driver)
    };
    // Retain the schema's complete member set even when hardware facts are withheld.
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
    fn large_scene_limits_preserve_defaults_and_check_both_exact_boundaries() {
        let small = BodyCount::try_from(1).unwrap();
        assert_eq!(
            required_limits(small, &wgpu::Limits::default()).unwrap(),
            wgpu::Limits::default()
        );
        let count = BodyCount::try_from(10000).unwrap();
        let bytes = 716_871_680;
        let available = wgpu::Limits {
            max_buffer_size: bytes,
            max_storage_buffer_binding_size: bytes,
            ..Default::default()
        };
        assert_eq!(required_limits(count, &available).unwrap(), available);
        for insufficient in [
            wgpu::Limits {
                max_buffer_size: bytes - 1,
                ..available.clone()
            },
            wgpu::Limits {
                max_storage_buffer_binding_size: bytes - 1,
                ..available
            },
        ] {
            assert!(matches!(
                required_limits(count, &insufficient),
                Err(BenchError::Unsupported(_))
            ));
        }
    }

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
