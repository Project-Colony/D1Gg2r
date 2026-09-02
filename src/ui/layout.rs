//! The measurements the whole interface is built from.
//!
//! Digger had six radii (including a lone `5.0`), fourteen paddings and
//! spacings ranging from 1 to 20 chosen case by case. Colony's chrome uses a
//! handful of values and nothing between them, which is what makes a window
//! read as one surface rather than as a pile of panels.
//!
//! These are not scaled by typography. A layout has to survive 1.68x text
//! rather than grow with it — see design/typography.md — and padding that grew
//! too would push the dashboard off the screen at the sizes that need it most.

/// Inside one item: a label stacked over its value.
pub(crate) const SPACE_2XS: f32 = 2.0;
/// Between items that belong to the same thought.
pub(crate) const SPACE_XS: f32 = 4.0;
/// Between rows of a list.
pub(crate) const SPACE_SM: f32 = 6.0;
/// Between controls in a group.
pub(crate) const SPACE_MD: f32 = 8.0;
/// Between groups.
pub(crate) const SPACE_LG: f32 = 12.0;
/// Between a heading and what it introduces.
pub(crate) const SPACE_XL: f32 = 20.0;

/// A card, a panel, a picker tile.
pub(crate) const RADIUS_CARD: f32 = 6.0;
/// A button, a selected row, an input.
pub(crate) const RADIUS_CONTROL: f32 = 8.0;

/// Inside a card or a panel.
pub(crate) const PAD_CARD: [f32; 2] = [10.0, 14.0];
/// A control or a list row: enough to click, not enough to waste a dashboard.
pub(crate) const PAD_ROW: [f32; 2] = [6.0, 12.0];
/// A badge, a chip, a table cell.
pub(crate) const PAD_TIGHT: [f32; 2] = [4.0, 8.0];
