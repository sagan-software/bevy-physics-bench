//! Owned GPU state, explicit completion, and pose readback for measured steps.

use crate::device::failure;
use bevy_physics_bench::{
    BenchError, BodySnapshot, InitialScene, QueryStatistics, SceneFingerprint, Settings, Snapshot,
};
use khal::backend::{Backend, GpuBackend};
use nexus_rbd3d::{
    math::Pose,
    pipeline::{RbdPipeline, RbdState},
};
use std::time::Duration;
use web_time::Instant;

/// Own one GPU scene and retain completed poses for rendering outside
/// measurement intervals.
/// This adapter accepts only its explicitly documented rigid-body
/// capability profile.
pub struct GpuSimulation {
    /// Device and submission queue that own the uploaded scene buffers.
    backend: GpuBackend,
    /// Compiled compute pipelines reused for every physics step.
    pipeline: RbdPipeline,
    /// Physical state retained on the device between completed steps.
    state: RbdState,
    /// Original geometry in creation order, including passive
    /// constraint anchors.
    scene: InitialScene,
    /// Reused readback storage includes the floor in slot zero.
    poses: Vec<Pose>,
    /// Completed simulation steps whose poses have reached the CPU.
    tick: u64,
    /// Measured wait and readback duration for the previous completed step.
    readback: Duration,
    /// Identity of initial geometry and settings, excluding evolving state.
    fingerprint: SceneFingerprint,
    /// Adapter-reported hardware facts captured before timing begins.
    metadata: serde_json::Value,
}

impl GpuSimulation {
    /// Validate a scene before requesting a GPU adapter or compiling
    /// compute pipelines.
    /// Unsupported modes return a capability error without allocating
    /// device resources.
    pub async fn new(settings: Settings) -> Result<Self, BenchError> {
        let (scene, fingerprint) = describe_validated(&settings)?;
        let (backend, metadata) = crate::device::create().await?;
        let pipeline = RbdPipeline::new(&backend).map_err(failure)?;
        let state = crate::scene::upload(&backend, &settings, &scene);
        let poses = read_initial_poses(&backend, &state).await?;
        Ok(Self {
            backend,
            pipeline,
            state,
            scene,
            poses,
            tick: 0,
            readback: Duration::ZERO,
            fingerprint,
            metadata,
        })
    }
    /// Submit one fixed simulation step and await its completed pose readback.
    /// Callers time this entire operation to avoid measuring submission
    /// latency alone.
    pub async fn advance(&mut self) -> Result<(), BenchError> {
        self.pipeline
            .step(&self.backend, &mut self.state, None)
            .map_err(failure)?;
        let start = Instant::now();
        // Reading poses establishes completion; the counter then checks this
        // exact step's capacity.
        self.backend
            .slow_read_buffer(self.state.body_poses().buffer(), &mut self.poses)
            .await
            .map_err(failure)?;
        let mut pairs = [0_u32];
        self.backend
            .slow_read_buffer(self.state.collision_pairs_len_gpu().buffer(), &mut pairs)
            .await
            .map_err(failure)?;
        self.readback = start.elapsed();
        validate_completed(&self.poses, pairs[0], self.state.collision_pairs_capacity())?;
        self.tick += 1;
        Ok(())
    }
    /// Copy completed poses into the shared render snapshot after the
    /// timed step ends.
    /// This method never starts GPU work or waits for another step.
    pub fn snapshot(&self) -> Snapshot {
        let bodies = self
            .scene
            .bodies()
            .iter()
            .zip(self.poses.iter().skip(1))
            .map(|(body, pose)| BodySnapshot {
                position: pose.translation.to_array(),
                rotation: pose.rotation.to_array(),
                shape: body.shape(),
                is_sleeping: false,
                kind: body.kind(),
            })
            .collect();
        let joints = self
            .scene
            .bodies()
            .iter()
            .enumerate()
            .filter_map(|(index, body)| body.joint().map(|joint| [joint.parent(), index]))
            .collect();
        Snapshot {
            tick: self.tick,
            bodies,
            joints,
            queries: QueryStatistics::default(),
        }
    }
    /// Return the previous step's elapsed readback and synchronization
    /// duration for separate reporting.
    /// It is part of the total step duration, not an additional charge.
    pub fn last_readback(&self) -> Duration {
        self.readback
    }
    /// Read adapter-reported hardware metadata captured from the device
    /// selected for this simulation.
    /// The native and browser collectors preserve these facts in
    /// exported results.
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
    /// Read the initial scene fingerprint without hashing evolving GPU
    /// state or allocating geometry.
    /// Compatible CPU runs use the same shared fingerprint profile.
    pub fn fingerprint(&self) -> SceneFingerprint {
        self.fingerprint
    }
}

impl std::fmt::Debug for GpuSimulation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GpuSimulation")
            .field("tick", &self.tick)
            .field("fingerprint", &self.fingerprint)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

/// Complete the initial upload before exposing an asynchronously stepped simulation.
async fn read_initial_poses(
    backend: &GpuBackend,
    state: &RbdState,
) -> Result<Vec<Pose>, BenchError> {
    // Complete the initial upload before exposing a usable simulation to
    // the caller.
    let mut poses =
        vec![Pose::IDENTITY; usize::try_from(state.body_poses().len()).map_err(failure)?];
    backend
        .slow_read_buffer(state.body_poses().buffer(), &mut poses)
        .await
        .map_err(failure)?;
    Ok(poses)
}

/// Reject capacity loss and nonfinite completed poses before advancing
/// the visible tick.
fn validate_completed(poses: &[Pose], pairs: u32, capacity: u32) -> Result<(), BenchError> {
    // Capacity loss invalidates the whole step rather than silently dropping contacts.
    if pairs > capacity {
        return Err(failure(std::io::Error::other(
            "Nexus collision-pair buffer overflow",
        )));
    }
    // Reject each nonfinite translation or rotation before exposing completed poses.
    let is_finite = poses.iter().all(is_finite_pose);
    if !is_finite {
        return Err(BenchError::NumericalFailure);
    }
    Ok(())
}

/// Require finite translation and rotation components in a completed GPU pose.
fn is_finite_pose(pose: &Pose) -> bool {
    pose.translation.is_finite() && pose.rotation.is_finite()
}

/// Validate the local capability profile before creating immutable initial inputs.
fn describe_validated(settings: &Settings) -> Result<(InitialScene, SceneFingerprint), BenchError> {
    crate::scene::validate(settings)?;
    Ok((InitialScene::new(settings), SceneFingerprint::of(settings)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_state_rejects_overflow_and_nonfinite_translation_or_rotation() {
        assert!(validate_completed(&[Pose::IDENTITY], 32, 32).is_ok());
        let error = validate_completed(&[Pose::IDENTITY], 33, 32).unwrap_err();
        assert!(matches!(error, BenchError::Engine(_)));
        assert!(std::error::Error::source(&error).is_some());
        let mut position = Pose::IDENTITY;
        position.translation.x = f32::NAN;
        assert!(matches!(
            validate_completed(&[position], 0, 32),
            Err(BenchError::NumericalFailure)
        ));
        let mut rotation = Pose::IDENTITY;
        rotation.rotation.x = f32::INFINITY;
        assert!(matches!(
            validate_completed(&[rotation], 0, 32),
            Err(BenchError::NumericalFailure)
        ));
    }
}
