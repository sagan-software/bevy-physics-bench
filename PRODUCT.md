# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Rust and Bevy simulations compile for native and WebAssembly targets.
A static browser interface runs on GitHub Pages.
The implementation chooses the frontend within the user's delegated design scope.

## Users

Bevy developers compare engine behavior, cost, and settings on their own hardware.

## Product purpose

Browse reproducible physics scenes, switch engines, customize settings, and run comparisons.
Publish measured results with engine versions, machine details, and benchmark settings.

## Capabilities and constraints

Support 2D and 3D simulations, CPU-only native measurement, and browser playback.
Keep native and browser measurements in separate series.
Keep shared settings and engine-specific solver settings visible.
Distinguish unavailable adapters from measured results.
Record new releases and accept community result files through pull requests.

## Evidence on hand

The user's Claude thread identified Avian, Rapier, Rolt/Jolt, and experimental Nexus.
The repository's research document records current compatibility and source links.
Every published timing must come from an actual run.
The implementation includes CPU browser execution and verified native GPU execution.
