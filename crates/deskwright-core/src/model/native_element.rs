use std::any::Any;
use std::fmt;
use std::sync::Arc;

/// An opaque handle to a platform UI element. The core passes it between ports without
/// knowing its type; only the adapter that created it can look inside.
#[derive(Clone)]
pub struct NativeElement(Arc<dyn Any + Send + Sync>);

impl NativeElement {
    pub fn new<PlatformElement: Any + Send + Sync>(platform_element: PlatformElement) -> Self {
        Self(Arc::new(platform_element))
    }

    pub fn downcast_ref<PlatformElement: Any>(&self) -> Option<&PlatformElement> {
        self.0.downcast_ref()
    }
}

impl fmt::Debug for NativeElement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NativeElement(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_creating_type_can_look_inside() {
        let element = NativeElement::new(7_u32);

        assert_eq!(element.downcast_ref::<u32>(), Some(&7));
        assert_eq!(element.downcast_ref::<i64>(), None);
    }

    #[test]
    fn debug_output_hides_the_platform_payload() {
        assert_eq!(
            format!("{:?}", NativeElement::new("secret")),
            "NativeElement(..)"
        );
    }
}
