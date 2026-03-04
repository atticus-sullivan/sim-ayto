// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This implements a trait, a normal + a mock implementation for working with a progressbar.

use indicatif::ProgressStyle;

/// can be used as a generic to make it configurable (during compile-time) which progressbar to use
pub trait ProgressBarTrait {
    /// increase the progressbar by `n`
    fn inc(&self, n: u64);
    /// signal the progressbar that it is finished
    fn finish(&self);
    /// set a `style` for the progressbar
    fn set_style(&self, style: ProgressStyle);
    /// create a new progressbar with a maxumum `num`
    fn new(num: u64) -> Self;
}

/// the generic progressbar which maps to the original indicatif progressbar
pub struct ProgressBar(indicatif::ProgressBar);
impl ProgressBarTrait for ProgressBar {
    fn inc(&self, n: u64) {
        self.0.inc(n)
    }

    fn finish(&self) {
        self.0.finish()
    }

    fn set_style(&self, style: ProgressStyle) {
        self.0.set_style(style)
    }

    fn new(num: u64) -> Self {
        ProgressBar(indicatif::ProgressBar::new(num))
    }
}

/// a mock-progressbar to avoid showing a progressbar
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MockProgressBar();
impl ProgressBarTrait for MockProgressBar {
    fn inc(&self, _n: u64) {}

    fn finish(&self) {}

    fn set_style(&self, _style: ProgressStyle) {}

    fn new(_num: u64) -> Self {
        MockProgressBar {}
    }
}
