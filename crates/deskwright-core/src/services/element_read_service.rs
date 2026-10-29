use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::DeskwrightResult;
use crate::model::element::{ElementRef, ElementSummary};
use crate::ports::ElementTreeReader;

/// Reads the live state of one element from the latest snapshot.
pub struct ElementReadService {
    registry: Arc<ElementRegistry>,
    tree: Arc<dyn ElementTreeReader>,
}

impl ElementReadService {
    pub fn new(registry: Arc<ElementRegistry>, tree: Arc<dyn ElementTreeReader>) -> Self {
        Self { registry, tree }
    }

    pub fn read(&self, element_ref: &ElementRef) -> DeskwrightResult<ElementSummary> {
        let element = self.registry.resolve(element_ref)?;
        Ok(ElementSummary {
            element_ref: *element_ref,
            attributes: self.tree.attributes(&element)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DeskwrightError;
    use crate::testing::{FakeDesktop, element};

    #[test]
    fn reads_current_attributes_of_a_registered_element() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let field = desktop.add_window(process_id, element("AXTextField", Some("Name")));
        let registry = Arc::new(ElementRegistry::default());
        let field_ref = desktop.register(&registry, field);
        desktop.set_value(field, "Ada");
        let service = ElementReadService::new(registry, desktop.clone());

        let summary = service.read(&field_ref).unwrap();

        assert_eq!(summary.attributes.value.as_deref(), Some("Ada"));
    }

    #[test]
    fn rejects_unknown_refs() {
        let desktop = Arc::new(FakeDesktop::new());
        let service = ElementReadService::new(Arc::new(ElementRegistry::default()), desktop);
        let unknown = ElementRef::from_index(9);

        assert_eq!(
            service.read(&unknown).unwrap_err(),
            DeskwrightError::UnknownElementRef(unknown)
        );
    }
}
