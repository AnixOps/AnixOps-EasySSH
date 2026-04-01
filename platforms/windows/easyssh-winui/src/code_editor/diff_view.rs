#![allow(dead_code)]

//! Diff View - Stub

/// Diff view
pub struct DiffView;

impl DiffView {
    pub fn new() -> Self {
        Self
    }

    pub fn compute_diff<T: Eq>(_old: &[T], _new: &[T]) -> Vec<DiffOp> {
        vec![]
    }
}

/// Diff operation
#[derive(Clone, Debug)]
pub enum DiffOp {
    Equal(usize),
    Insert(usize),
    Delete(usize),
}
