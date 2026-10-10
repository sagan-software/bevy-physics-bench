//! Apply user interactions through the selected engine's velocity components.

use super::Adapter;
use crate::{BenchError, BodyKick, BodyKind, Dimension, Engine};

impl Adapter {
    /// Read the current count from the retained entity mapping.
    pub(crate) fn body_count(&self) -> crate::BodyCount {
        crate::BodyCount::try_from(self.bodies.len() as u32).expect("bounded body mapping")
    }

    /// Retain appended primitives in the same stable order as the initial scene.
    pub(crate) fn append(
        &mut self,
        settings: &crate::Settings,
        bodies: &[crate::InitialBody],
    ) -> Result<(), BenchError> {
        // Retain material and collision policies for every appended primitive.
        let input = settings.input();
        // Native Jolt owns its registry outside ECS; other backends consume components.
        if self.engine == Engine::Jolt {
            #[cfg(all(feature = "jolt", not(target_arch = "wasm32")))]
            {
                let entities = super::jolt::append(&mut self.app, &input, bodies)?;
                self.bodies
                    .extend(entities.into_iter().zip(bodies.iter().copied()));
                return Ok(());
            }
            #[cfg(not(all(feature = "jolt", not(target_arch = "wasm32"))))]
            return Err(BenchError::Unsupported("native Jolt is unavailable"));
        }
        // Append stable mappings only after their engine entity has been created.
        for body in bodies {
            let world = self.app.world_mut();
            let entity = match (self.engine, self.dimension) {
                (Engine::Avian, Dimension::Two) => super::avian2::spawn_body(world, &input, body),
                (Engine::Avian, Dimension::Three) => super::avian3::spawn_body(world, &input, body),
                (Engine::Rapier, Dimension::Two) => super::rapier2::spawn_body(world, &input, body),
                (Engine::Rapier, Dimension::Three) => {
                    super::rapier3::spawn_body(world, &input, body)
                }
                _ => return Err(BenchError::Unsupported("growth requires a physics engine")),
            };
            self.bodies.push((entity, *body));
        }
        Ok(())
    }
    /// Validate target ownership before changing velocity or waking a body.
    pub(crate) fn kick(&mut self, kick: BodyKick) -> Result<(), BenchError> {
        // Validate ownership and dimension before selecting an engine mutation.
        let entity = self.kick_target(kick)?;
        let delta = kick.delta();
        // Each integration owns its velocity and sleeping state.
        let world = self.app.world_mut();
        match (self.engine, self.dimension) {
            (Engine::Avian, Dimension::Two) => super::avian2::kick(world, entity, delta),
            (Engine::Avian, Dimension::Three) => super::avian3::kick(world, entity, delta),
            (Engine::Rapier, Dimension::Two) => super::rapier2::kick(world, entity, delta),
            (Engine::Rapier, Dimension::Three) => super::rapier3::kick(world, entity, delta),
            (Engine::Jolt, _) => {
                #[cfg(all(feature = "jolt", not(target_arch = "wasm32")))]
                return super::jolt::kick(world, kick);
                #[cfg(not(all(feature = "jolt", not(target_arch = "wasm32"))))]
                return Err(BenchError::Unsupported("native Jolt is unavailable"));
            }
            (Engine::Bevy, _) => Err(BenchError::Unsupported(
                "animation does not accept physics kicks",
            )),
        }
    }

    /// Resolve one dynamic target and reject out-of-plane velocity before dispatch.
    fn kick_target(&self, kick: BodyKick) -> Result<bevy::prelude::Entity, BenchError> {
        // Validate target identity before accessing backend state.
        let &(entity, body) = self
            .bodies
            .get(kick.index())
            .ok_or(BenchError::InvalidSetting("kick target is absent"))?;
        if body.kind != BodyKind::Dynamic || self.engine == Engine::Bevy {
            return Err(BenchError::Unsupported(
                "kick target must be a dynamic physics body",
            ));
        }
        // Reject out-of-plane input instead of silently dropping its Z component.
        let delta = kick.delta();
        if self.dimension == Dimension::Two && delta.z != 0.0 {
            return Err(BenchError::InvalidSetting(
                "2D kick must have zero Z velocity",
            ));
        }
        Ok(entity)
    }
}
