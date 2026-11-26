# Changelog

## Unreleased

### Added

- Additional builds for varying CPU-supported features (sse2, sse4.2, avx2, avx512)

- Core damage systems and prototype user interactions

- Delete useless cages after associated fragment is deleted

- Visualize cursor 3D position in world (spatial hash grid)

- Create interface panel to interact with internal profiler

- GUI: add option to orient slider widgets vertically

- GUI: `<Widget>Options` builder structs for widget parameters

- GUI: new 'Looks' systems to define broader widget appearance/styling

- GUI: macro helpers to generate layout properties (various enums, relative/absolute values, points, rectangles...)

- GUI: new hierarchical UI builder to define UI trees

- GUI: implemented new 'checkbox' widget

- GUI: added new `visible` field to common widget data, applicable to all widgets.  
  `visible` set to `true` will effectively set the internal widget's style `display` property to `none`.

- GUI: created new `Property` type for less tedious UI-state synchronisation

- GUI: implemented widget removal/deletion

- GUI: implemented new 'dropdown-menu' widget

- GUI: create `gui_macros::wrap_property!` procedural macro to generate typed `Property<T>` types

- GUI: create `derive(gui_macros::DropdownOptions)` procedural macro to generate `HasDropdownOptions` impl for enum types.

### Changed

- Updated `build.sh` build script to support additional builds

- Profiler can now be enabled/disabled, rather than be running at all times

- GUI: add `hidden` property to panel widgets

- GUI: remove all color-related properties from widget options

- Reworked geometry submission path for fragments:  
  Fragments that are not actively deforming are rendered through instancing, like debris.  
  Fragments are only submitted as dynamic geometry if their deformation displacement is above a certain 
  threshold.  
  This severely improves performance: as it greatly reduces vertex fetching cost during dynamic geometry 
  rasterization.

- GUI: unconstrained `LAYERS` generic constant parameter from `BatchingLayerCompositor` and `InterfaceSystem`  
  The layers list of `BatchingLayerCompositor` is now heap-allocated

- GUI: added `*WithData` variants to `InteractionCallback` enum for runtime selection/mutation of env IDs and
  increased flexibility for interactable elements.

### Fixed

- Crash during debris simulation due to missing check(s)

- Restored debug lines rendering under `devmode` feature flag

- GUI: correctly select only the top-most floating panel when grabbing a panel that overlays another floating panel, avoiding conflict

## 0.1.0 - 2026/10/29
Base pre-changelog state
