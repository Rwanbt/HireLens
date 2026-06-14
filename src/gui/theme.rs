//! Design tokens for the HireLens GUI — the single source of truth for colors,
//! radii, and spacing. Views and widgets must reference these constants and
//! never inline hex values or magic spacing numbers.
//!
//! Palette: "HireLens Blue" dark theme. The status colors keep the exact RGB
//! values used since the first GUI release, so this module can replace the old
//! `COL_*` constants without any visual regression; the new identity comes from
//! the darker app/card backgrounds wired in [`super::custom_visuals`].

// WHY: design-token registry — every token is declared up-front (see plan
// Phase 10 / P1.1) and wired in incrementally across the redesign phases
// P1–P6. The blanket allow is removed once every token is referenced (end P6).
#![allow(dead_code)]

use eframe::egui::Color32;

// ── Backgrounds ──
pub(crate) const BG_APP: Color32 = Color32::from_rgb(0x0D, 0x11, 0x17);
pub(crate) const BG_CARD: Color32 = Color32::from_rgb(0x16, 0x1B, 0x22);
pub(crate) const BG_HOVER: Color32 = Color32::from_rgb(0x21, 0x26, 0x2D);

// ── Borders ──
pub(crate) const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x30, 0x36, 0x3D);
pub(crate) const BORDER_ACTIVE: Color32 = Color32::from_rgb(0x1F, 0x6F, 0xEB);

// ── Text (three levels of hierarchy) ──
pub(crate) const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xE6, 0xED, 0xF3);
pub(crate) const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x8B, 0x94, 0x9E);
pub(crate) const TEXT_MUTED: Color32 = Color32::from_rgb(0x6E, 0x76, 0x81);

// ── Accent — HireLens Blue ──
pub(crate) const ACCENT_PRIMARY: Color32 = Color32::from_rgb(0x2D, 0x6E, 0xE8);

// ── Status ──
pub(crate) const STATUS_SUCCESS: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x50);
pub(crate) const STATUS_WARNING: Color32 = Color32::from_rgb(0xD2, 0x99, 0x22);
pub(crate) const STATUS_ORANGE: Color32 = Color32::from_rgb(0xFB, 0x92, 0x3C);
pub(crate) const STATUS_ERROR: Color32 = Color32::from_rgb(0xF8, 0x51, 0x49);
pub(crate) const STATUS_INFO: Color32 = Color32::from_rgb(0x58, 0xA6, 0xFF);

// ── Corner radii ──
pub(crate) const RADIUS_SM: f32 = 4.0;
pub(crate) const RADIUS_MD: f32 = 6.0;
pub(crate) const RADIUS_LG: f32 = 10.0;

// ── Spacing / gaps ──
pub(crate) const GAP_SM: f32 = 6.0;
pub(crate) const GAP_MD: f32 = 12.0;
pub(crate) const GAP_LG: f32 = 24.0;
pub(crate) const PAD_CARD: f32 = 16.0;
