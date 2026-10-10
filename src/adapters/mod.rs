//! Bevy engine adapters for shared scene and stepping contracts.

use crate::{BodySnapshot, Dimension, Engine, Settings, scene::Body};
use bevy::{prelude::*, time::TimeUpdateStrategy};

mod avian2;
mod avian3;
mod ik;
mod interaction;
mod rapier2;
mod rapier3;

#[cfg(all(feature = "jolt", not(target_arch = "wasm32")))]
mod jolt;

/// A headless Bevy app with engine bodies in stable scene order.
pub(crate) struct Adapter {
    /// Bevy owns the engine resources and schedules.
    app: App,
    /// Scene entities paired with their geometry, excluding the floor.
    bodies: Vec<(Entity, Body)>,
    /// Engine used to interpret sleeping state.
    engine: Engine,
    /// Dimension used to interpret sleeping state.
    dimension: Dimension,
}

/// Mark a prescribed moving platform for the per-engine driver.
#[derive(Component)]
pub(super) struct Driven;

/// Mark controller bodies whose translation comes from a shape cast.
#[derive(Component)]
pub(super) struct Character;

/// Fixed ray origins derived once from the shared initial body positions.
#[derive(Resource)]
pub(super) struct QueryOrigins(Vec<Vec3>);

/// Retain a backend failure until the public step operation returns it.
#[derive(Default, Resource)]
pub(super) struct EngineStatus(Option<crate::BenchError>);

impl Adapter {
    /// Initialize one real physics integration and its shared scene.
    pub(crate) fn new(
        engine: Engine,
        settings: &Settings,
        bodies: &[Body],
        capacity: crate::BodyCount,
        floor: Vec3,
    ) -> Result<Self, crate::BenchError> {
        let is_ik = settings.input().scenario == crate::Scenario::InverseKinematics;
        if is_ik != (engine == Engine::Bevy) {
            return Err(crate::BenchError::Unsupported(
                "inverse kinematics uses the Bevy animation series only",
            ));
        }
        // A private app owns schedules and resources for this simulation alone.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(settings.timestep()));
        app.init_resource::<crate::QueryStatistics>();
        app.init_resource::<EngineStatus>();
        insert_query_origins(&mut app, bodies);
        // Build the selected backend before finishing plugin initialization.
        let dimension = settings.input().dimension;
        let entities = build_engine(
            &mut app, engine, dimension, settings, bodies, capacity, floor,
        )?;
        app.finish();
        app.cleanup();
        // Rapier initializes its context outside the measured schedule.
        if engine == Engine::Rapier {
            app.world_mut()
                .try_run_schedule(PreStartup)
                .map_err(integration_error)?;
        }
        // Keep geometry paired with the entity created from it for later snapshots.
        Ok(Self {
            app,
            bodies: entities.into_iter().zip(bodies.iter().copied()).collect(),
            engine,
            dimension,
        })
    }

    /// Run exactly one update with the configured simulation duration.
    pub(crate) fn step(&mut self, settings: &Settings) -> Result<(), crate::BenchError> {
        // Advance directly to avoid startup's zero-duration frame.
        self.app
            .world_mut()
            .get_resource_mut::<Time>()
            .ok_or_else(|| {
                integration_error(std::io::Error::other("Bevy time resource is absent"))
            })?
            .advance_by(settings.timestep());
        self.app
            .world_mut()
            .try_run_schedule(PostUpdate)
            .map_err(integration_error)?;
        if let Some(error) = self
            .app
            .world_mut()
            .get_resource_mut::<EngineStatus>()
            .and_then(|mut status| status.0.take())
        {
            return Err(error);
        }
        Ok(())
    }

    /// Copy poses after timing has stopped.
    pub(crate) fn snapshot(&self) -> Vec<BodySnapshot> {
        self.bodies
            .iter()
            .map(|(entity, body)| {
                let world = self.app.world();
                let transform = world.get::<Transform>(*entity).expect("scene body exists");
                let is_sleeping = self.is_sleeping(*entity);
                BodySnapshot {
                    position: transform.translation.to_array(),
                    rotation: transform.rotation.to_array(),
                    shape: body.shape,
                    is_sleeping,
                    kind: body.kind,
                }
            })
            .collect()
    }

    /// Read only the sleeping marker owned by the selected engine integration.
    fn is_sleeping(&self, entity: Entity) -> bool {
        let world = self.app.world();
        match (self.engine, self.dimension) {
            (Engine::Avian, Dimension::Two) => {
                world.get::<avian2d::prelude::Sleeping>(entity).is_some()
            }
            (Engine::Avian, Dimension::Three) => {
                world.get::<avian3d::prelude::Sleeping>(entity).is_some()
            }
            (Engine::Rapier, Dimension::Two) => world
                .get::<bevy_rapier2d::prelude::Sleeping>(entity)
                .is_some_and(|state| state.sleeping),
            (Engine::Rapier, Dimension::Three) => world
                .get::<bevy_rapier3d::prelude::Sleeping>(entity)
                .is_some_and(|state| state.sleeping),
            (Engine::Jolt, _) => world
                .get::<JoltSleeping>(entity)
                .is_some_and(|state| state.0),
            (Engine::Bevy, _) => false,
        }
    }

    /// Read stable body-index pairs for the joint overlay.
    pub(crate) fn joints(&self) -> Vec<[usize; 2]> {
        self.bodies
            .iter()
            .enumerate()
            .filter_map(|(index, (_, body))| body.joint.map(|joint| [joint.parent, index]))
            .collect()
    }

    /// Return the observable workload counters without entering a timed query.
    pub(crate) fn queries(&self) -> crate::QueryStatistics {
        self.app
            .world()
            .get_resource::<crate::QueryStatistics>()
            .copied()
            .unwrap_or_default()
    }
}

/// Preserve a backend error as the source of the public integration failure.
fn integration_error(error: impl std::error::Error + Send + Sync + 'static) -> crate::BenchError {
    crate::BenchError::Engine(Box::new(error))
}

/// Store Jolt's deactivation state beside its synchronized render transform.
#[derive(Component)]
pub(super) struct JoltSleeping(bool);

/// Resolve a generated joint parent while preserving an explicit error for
/// a malformed scene.
fn joint_parent(entities: &[Entity], index: usize) -> Result<Entity, crate::BenchError> {
    entities
        .get(index)
        .copied()
        .ok_or_else(|| integration_error(std::io::Error::other("generated joint parent is absent")))
}

/// Select the supported engine integration before any timed schedule can run.
fn build_engine(
    app: &mut App,
    engine: Engine,
    dimension: Dimension,
    settings: &Settings,
    bodies: &[Body],
    capacity: crate::BodyCount,
    floor: Vec3,
) -> Result<Vec<Entity>, crate::BenchError> {
    let _ = capacity; // Native Jolt allocates its bounded body registry up front.
    match (engine, dimension) {
        (Engine::Avian, Dimension::Two) => avian2::build(app, settings, bodies, floor),
        (Engine::Avian, Dimension::Three) => avian3::build(app, settings, bodies, floor),
        (Engine::Rapier, Dimension::Two) => rapier2::build(app, settings, bodies, floor),
        (Engine::Rapier, Dimension::Three) => rapier3::build(app, settings, bodies, floor),
        (Engine::Bevy, _) => Ok(ik::build(app, settings, bodies)),
        (Engine::Jolt, Dimension::Two) => Err(crate::BenchError::Unsupported(
            "Jolt supports only 3D scenes",
        )),
        (Engine::Jolt, Dimension::Three) => {
            #[cfg(all(feature = "jolt", not(target_arch = "wasm32")))]
            {
                jolt::build(app, settings, bodies, capacity, floor)
            }
            #[cfg(not(all(feature = "jolt", not(target_arch = "wasm32"))))]
            {
                return Err(crate::BenchError::Unsupported(
                    "Jolt requires its native feature or the browser JavaScript adapter",
                ));
            }
        }
    }
}

/// Fix ray inputs before any physics update can move their source bodies.
fn insert_query_origins(app: &mut App, bodies: &[Body]) {
    // Alternate body hits with displaced misses; short rays cannot hit the common floor.
    app.insert_resource(QueryOrigins(
        bodies
            .iter()
            .enumerate()
            .map(|(index, body)| {
                Vec3::new(
                    body.position.x + if index % 2 == 0 { 0.0 } else { 1_000.0 },
                    body.position.y + 1.0,
                    body.position.z,
                )
            })
            .collect(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn joint_parent_rejects_absent_entities_and_returns_present_entities() {
        let entity = Entity::from_bits(1);
        assert_eq!(joint_parent(&[entity], 0).unwrap(), entity);
        assert!(joint_parent(&[], 0).is_err());
    }
}
