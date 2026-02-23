/// This implements a trait, a normal + a mock implementation for working with a progressbar.
use indicatif::ProgressStyle;

pub trait ProgressBarTrait {
    fn inc(&self, n: u64);
    fn finish(&self);
    fn set_style(&self, style: ProgressStyle);
    fn new(num: u64) -> Self;
}

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
