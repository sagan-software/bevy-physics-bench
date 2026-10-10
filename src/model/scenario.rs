//! Stable benchmark identifiers and supported dimensions.

use serde::{Deserialize, Serialize};

/// A closed collection of scene workloads with shared initial geometry and
/// settings. Each engine retains its own solver behavior while receiving the
/// same generated inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    /// Separated unit boxes falling under gravity into a pile on the common
    /// floor. A seeded horizontal jitter prevents every initial contact from
    /// sharing identical alignment.
    #[default]
    Boxes,
    /// Separated balls falling under gravity into a pile on the common floor.
    /// Each radius is half a metre; planar engines use circles and spatial
    /// engines use spheres.
    Balls,
    /// A triangular arrangement of touching unit boxes resting on the common
    /// floor. Bodies are created left to right and bottom to top, retaining
    /// exactly the requested count.
    Pyramid,
    /// Separate columns containing at most twelve unit boxes each, spaced
    /// vertically by one hundredth of a metre. A partial final column retains
    /// the requested body count.
    Stacks,
    /// A shared lattice with one dynamic body for every four requested bodies.
    /// The other bodies remain fixed, exercising contacts and broad-phase work
    /// among mostly static geometry.
    StaticField,
    /// A shared lattice with every eighth body replaced by a moving platform.
    /// Its horizontal velocity follows simulation time while the remaining
    /// bodies respond dynamically to contacts.
    Kinematics,
    /// A row of thin boxes with the first box given an initial angular
    /// velocity. Contact propagation topples the row without applying further
    /// scripted impulses during simulation.
    Dominoes,
    /// Independent chains containing at most twelve box links, with each first
    /// link fixed. Adjacent links share point anchors and suppress their mutual
    /// contacts while other contacts remain enabled.
    JointChain,
    /// Passive humanoids formed from a torso, head, arms, and legs connected by
    /// point constraints. Partial final humanoids preserve the requested count
    /// without adding motors or joint limits.
    Ragdolls,
    /// Small balls moving downward at one hundred twenty metres per second
    /// toward a thin floor. Discrete and continuous collision policies expose
    /// their different tunneling behavior.
    Ccd,
    /// Cast one short downward ray per static body after every completed step.
    /// Alternating rays target a body or a displaced empty position, exercising
    /// hits and misses.
    Queries,
    /// Move spherical character controllers through fixed ramps using each
    /// engine collision-and-slide implementation. Requested horizontal velocity
    /// follows simulation time while downward velocity keeps controllers
    /// against surfaces.
    Characters,
    /// Analytic two-bone inverse kinematics measured as Bevy animation work in
    /// a separate series. Each complete limb updates its elbow and endpoint
    /// without creating a physics world.
    InverseKinematics,
}

impl std::str::FromStr for Scenario {
    type Err = crate::BenchError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let text = serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value);
        Self::deserialize(text).map_err(crate::BenchError::Vocabulary)
    }
}
