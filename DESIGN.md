---
name: Bevy physics bench
description: Restrained controls and measured results around dark physics stages.
colors:
  primary: "#216b5e"
  primary-hover: "#19584d"
  text: "#253039"
  muted: "#5e6a70"
  neutral-bg: "#f6f7f5"
  panel: "#fff"
  line: "#d9dfdc"
  focus: "#176d9b"
  control-border: "#b9c4bd"
  control-hover: "#eaf0ec"
  control-hover-border: "#8d9e93"
  scene-bg: "#f0f3ef"
  scene-active-bg: "#dcebe1"
  scene-active-text: "#174f43"
  engine-select-bg: "#f1f4f1"
  stage: "#14212b"
  stage-footer: "#192a35"
  stage-text: "#c5d3db"
  stage-divider: "#3d4f5a"
  stage-border: "#cad3cd"
  avian-indicator: "#257d66"
  rapier-indicator: "#9d6b25"
  avian-metric: "#2b8570"
  rapier-metric: "#a97835"
  jolt-indicator: "#647bbb"
  bevy-indicator: "#697d88"
  avian-body: "#7cdbbc"
  rapier-body: "#e8b776"
  nexus-body: "#bfa0d8"
  bevy-body: "#c1c9cc"
  jolt-body: "#aabcf0"
  error-bg: "#fff0e9"
  error-text: "#873a22"
  error-border: "#e2b6a5"
typography:
  title:
    fontFamily: 'ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif'
    fontSize: "1.5rem"
    fontWeight: 650
    letterSpacing: "-.025em"
  section:
    fontSize: "1rem"
    fontWeight: 650
  body:
    fontSize: ".875rem"
    lineHeight: 1.5
  control:
    fontSize: ".875rem"
    fontWeight: 550
  label:
    fontSize: ".8125rem"
    fontWeight: 550
  compact-control:
    fontSize: ".75rem"
  metadata:
    fontSize: ".6875rem"
  document-body:
    fontSize: ".9375rem"
    lineHeight: 1.65
rounded:
  control: "5px"
  simulation: "7px"
spacing:
  control-gap: "6px"
  toolbar-gap: "7px"
  action-gap: "8px"
  compact-gap: "12px"
  settings-gap: "14px"
  mobile-settings-gap: "15px"
  tablet-settings-gap: "16px"
  compact-layout-gap: "18px"
  experiment-gap: "26px"
  wide-layout-gap: "32px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.panel}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: "7px 12px"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.panel}"
  button-secondary:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: "7px 12px"
  button-secondary-hover:
    backgroundColor: "{colors.control-hover}"
  button-text:
    textColor: "{colors.primary}"
    typography: "{typography.compact-control}"
    padding: "2px 0"
  field:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "6px 9px"
  navigation:
    textColor: "{colors.muted}"
  navigation-current:
    textColor: "{colors.primary}"
  scene-current:
    backgroundColor: "{colors.scene-active-bg}"
    textColor: "{colors.scene-active-text}"
    padding: "9px 11px"
  simulation:
    backgroundColor: "{colors.stage}"
    rounded: "{rounded.simulation}"
  error:
    backgroundColor: "{colors.error-bg}"
    textColor: "{colors.error-text}"
    rounded: "{rounded.control}"
    padding: "13px"
---

# Design System: Bevy physics bench

## Overview

Creative North Star: "Physics workbench"

The interface gives Bevy developers direct access to scenes, engine selection, playback, settings, and measured results.
A light application frame separates readable controls from dark simulation stages.
Compact labels and tabular measurements support comparison without decorative panels.

The user directed this restrained application through the code-led Operate workflow.
The confirmed direction has no concept roll or FORM seed.
The playground's composition belongs to `.impeccable/surfaces/playground.md`; its exact layout is not a rule for every future page.

Key Characteristics:

- Light controls around dark simulations.
- Engine names accompany engine colors.
- Borders and spacing separate tasks.
- Simulation provides the visible motion.

Tokens above record `web/src/style.css` and engine colors in `web/src/types.ts`.
The sidecar records component examples and responsive rules.
Its generated tonal ramps are panel previews; they are not implemented color tokens.

## Colors

Teal identifies primary actions and current navigation.
Engine indicators, timing bars, and simulation bodies use separate color values for their different backgrounds.

Primary uses `primary` and `primary-hover` for the comparison action, links, and selected main navigation.
Avian uses the `avian-indicator`, `avian-metric`, and `avian-body` tokens.
Rapier uses the corresponding amber tokens.
Jolt uses blue; Bevy IK uses gray; Nexus simulation bodies use purple.
The implementation has no separate Nexus indicator rule.

Neutral tokens distinguish the application background, white controls, scene navigation, dividers, and muted text.
Dark stage tokens distinguish the canvas background, live-status footer, and stage separators.
Error tokens identify the alert without replacing its explanation or recovery action.

The engine identity rule. Keep the engine name visible wherever color identifies an engine.

## Typography

The system sans-serif stack serves application headings, controls, labels, and metadata.
The title token describes the scene heading; the mobile scene heading becomes `1.375rem`.
Section headings use the section token; settings headings use `.875rem`.
Labels use the label token; playback controls use compact-control.
Metadata uses the metadata token for versions, live-status footers, timing labels, and table headings.
Document prose uses document-body and a maximum width of `72ch`.

Measurements, numeric fields, engine versions, and result cells use `font-variant-numeric:tabular-nums`.
Scene descriptions use a maximum width of `68ch`.

## Layout

The playground uses a scene navigation column beside a fluid workspace.
The default scene column is `190px`; the settings column is `204px` with the experiment-gap token.
The workspace uses `26px 28px 32px` padding.
The simulations occupy two equal columns.
Document pages use a separate centered container with a maximum width of `1200px`.

When the viewport is at least `1550px`, the scene column becomes `215px` and the settings column becomes `225px`.
When the viewport is at least `1550px`, workspace padding becomes `32px 40px` and the experiment gap becomes `32px`.
When the viewport is at most `1180px`, the scene column becomes `165px` and the settings column becomes `180px`.
When the viewport is at most `1180px`, workspace padding becomes `22px 18px` and the experiment gap becomes `18px`.
When the viewport is at most `930px`, settings move below the simulations into four columns.

When the viewport is at most `680px`, the scene selector replaces the scene navigation column.
When the viewport is at most `680px`, simulations stack and settings use two columns.
When the viewport is at most `680px`, playback precedes the simulations through `.simulation-area>.playback{order:-1;margin-bottom:12px}`.
When the viewport is at most `680px`, workspace padding becomes `20px 14px` and main navigation occupies a second header row.
Result tables scroll horizontally within `.table-scroll`; result filters wrap.

Stage height is `390px` by default, `480px` at the wide breakpoint, `360px` at the compact breakpoint, and `350px` below `930px`.
Stage height becomes `285px` below `680px`.
These measurements describe the playground, not a universal screen template.

## Elevation & Depth

The interface has no CSS shadows.
Surface tones and single-pixel borders separate controls, navigation, simulations, and results.
The dark stages contain rendered depth; surrounding controls remain flat.
The stylesheet defines no decorative transitions or keyframe animation.
The reduced-motion rule sets `scroll-behavior:auto!important`.

The flat controls rule. Separate application controls with tones, borders, and spacing; preserve the dark simulation stage.

## Shapes

Controls and alerts use the control radius; the joined simulation container uses the simulation radius.
Simulation corners clip their contents through `overflow:hidden`.
Engine indicators are circles measuring `7px` in each dimension.
Navigation uses text and a thin current-page underline.
Result sections use dividers rather than a card grid.

## Components

Buttons use compact padding and a `36px` minimum height.
Primary buttons use the primary color; secondary buttons use white.
Enabled hover states change background and border color.
Disabled buttons use `opacity:.5` and `cursor:not-allowed`.
Playback buttons use `5px 10px` padding, a `50px` minimum width, and a `33px` minimum height.
The Defaults text button removes the border and uses the primary color.

Inputs and selects share control borders, radius, and height.
Labels place a `6px` gap before the control.
Paired fields remain distinct labeled inputs.
The focus rule uses `outline:3px solid var(--focus)` with `outline-offset:3px` on buttons, links, fields, summaries, and canvases.

Main navigation marks the current page with primary-colored text, weight `650`, and a `2px` bottom border.
Scene navigation marks the current scene with the scene-active background and text tokens.
Source remains a text link.

Simulation toolbars combine an engine indicator, engine name, selector, and compact version text.
Their default height is `56px`; mobile uses `48px`.
Engine selects flex within the available width; version labels do not shrink.
The current metadata uses compact versions, including `JS 1.1.0` for Jolt.
Live-status footers use tabular values on the dark footer background.

Results use left-aligned tables with muted headers, tabular values, and `4px` timing bars.
Expandable sections hold hardware, versions, settings, and sampling details.
Alerts use the error tokens, `role="alert"`, and explicit recovery text.
Measurement status uses `role="status"`; progress uses the primary color.

## Do's and Don'ts

- Do preserve visible engine names beside engine colors.
- Do keep numeric measurements tabular and retain runtime, version, and sampling context.
- Do keep keyboard focus visible on controls, links, summaries, and canvases.
- Do preserve the playground's mobile playback order and reachable scene selector.
- Don't add promotional copy, decorative animation, or nested panels to the workbench.
- Don't present unavailable adapters or empty result states as measured results.
- Don't turn playground column widths or stage heights into layout rules for every page.
- Don't use generated tonal ramps as implemented tokens without an approved design change.
