use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::element::ElementRef;
use crate::model::native_element::NativeElement;

/// Maps the refs handed out by the latest snapshot to their platform elements, and hands out
/// ref numbers that keep increasing, so refs from older snapshots always fail to resolve.
#[derive(Debug)]
pub struct ElementRegistry {
    state: Mutex<RegistryState>,
}

#[derive(Debug)]
struct RegistryState {
    elements_by_ref: HashMap<ElementRef, NativeElement>,
    next_index: usize,
}

const FIRST_REF_INDEX: usize = 1;

impl Default for ElementRegistry {
    fn default() -> Self {
        Self {
            state: Mutex::new(RegistryState {
                elements_by_ref: HashMap::new(),
                next_index: FIRST_REF_INDEX,
            }),
        }
    }
}

impl ElementRegistry {
    /// The number the next snapshot's first element should get.
    pub fn next_index(&self) -> usize {
        self.locked().next_index
    }

    /// Atomically replaces every registered element, invalidating refs from older snapshots.
    /// Later snapshots are numbered after the highest ref registered here.
    pub fn replace_all(&self, elements_by_ref: HashMap<ElementRef, NativeElement>) {
        let mut state = self.locked();
        let after_highest = elements_by_ref
            .keys()
            .map(|element_ref| element_ref.index() + 1)
            .max()
            .unwrap_or(FIRST_REF_INDEX);
        state.next_index = state.next_index.max(after_highest);
        state.elements_by_ref = elements_by_ref;
    }

    pub fn resolve(&self, element_ref: &ElementRef) -> DeskwrightResult<NativeElement> {
        self.locked()
            .elements_by_ref
            .get(element_ref)
            .cloned()
            .ok_or(DeskwrightError::UnknownElementRef(*element_ref))
    }

    // Every critical section is a single assignment or lookup, so a panic elsewhere cannot
    // leave the state half-updated; recovering from poisoning is therefore safe.
    fn locked(&self) -> MutexGuard<'_, RegistryState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_invalidates_refs_from_the_previous_snapshot() {
        let registry = ElementRegistry::default();
        let first_ref = ElementRef::from_index(1);
        registry.replace_all(HashMap::from([(first_ref, NativeElement::new(1_u8))]));
        assert!(registry.resolve(&first_ref).is_ok());

        registry.replace_all(HashMap::new());

        assert_eq!(
            registry.resolve(&first_ref).unwrap_err(),
            DeskwrightError::UnknownElementRef(first_ref)
        );
    }

    #[test]
    fn numbers_keep_increasing_after_each_replacement() {
        let registry = ElementRegistry::default();
        assert_eq!(registry.next_index(), 1);

        registry.replace_all(HashMap::from([(
            ElementRef::from_index(5),
            NativeElement::new(5_u8),
        )]));
        assert_eq!(registry.next_index(), 6);

        registry.replace_all(HashMap::new());
        assert_eq!(registry.next_index(), 6);
    }
}
