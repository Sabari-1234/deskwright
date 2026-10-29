use crate::error::DeskwrightResult;
use crate::model::app::ProcessId;
use crate::model::element::ElementAttributes;
use crate::model::geometry::Bounds;
use crate::model::native_element::NativeElement;

/// An element's attributes together with its children.
#[derive(Debug, Clone)]
pub struct ElementContents {
    pub attributes: ElementAttributes,
    pub children: Vec<NativeElement>,
}

/// Reads an app's UI element tree.
pub trait ElementTreeReader: Send + Sync {
    /// The app's windows, or its direct children when it has no windows.
    fn top_level_elements(&self, process_id: ProcessId) -> DeskwrightResult<Vec<NativeElement>>;

    fn attributes(&self, element: &NativeElement) -> DeskwrightResult<ElementAttributes>;

    /// The element's children; empty for leaf elements.
    fn children(&self, element: &NativeElement) -> DeskwrightResult<Vec<NativeElement>>;

    /// Attributes and children in one call. Adapters should override this to fetch both in a
    /// single request: tree walks call it for every element, so it dominates their speed.
    fn read_with_children(&self, element: &NativeElement) -> DeskwrightResult<ElementContents> {
        Ok(ElementContents {
            attributes: self.attributes(element)?,
            children: self.children(element)?,
        })
    }

    /// The element's bounds, or `None` when it is off-screen or has zero size.
    fn on_screen_bounds(&self, element: &NativeElement) -> DeskwrightResult<Option<Bounds>> {
        Ok(self
            .attributes(element)?
            .bounds
            .filter(|bounds| !bounds.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_registry::ElementRegistry;
    use crate::testing::{FakeDesktop, element};

    fn bounds_of(width: f64, height: f64) -> DeskwrightResult<Option<Bounds>> {
        let desktop = FakeDesktop::new();
        let process_id = desktop.add_app("Notes", None);
        let mut attributes = element("AXGroup", None);
        attributes.bounds = Some(Bounds {
            x: 1.0,
            y: 2.0,
            width,
            height,
        });
        let group = desktop.add_window(process_id, attributes);
        let registry = ElementRegistry::default();
        let group_ref = desktop.register(&registry, group);
        let handle = registry.resolve(&group_ref)?;
        desktop.on_screen_bounds(&handle)
    }

    #[test]
    fn returns_bounds_with_a_visible_area() {
        assert_eq!(
            bounds_of(10.0, 5.0).unwrap().map(|bounds| bounds.width),
            Some(10.0)
        );
    }

    #[test]
    fn treats_zero_sized_bounds_as_not_on_screen() {
        assert_eq!(bounds_of(0.0, 5.0).unwrap(), None);
    }
}
