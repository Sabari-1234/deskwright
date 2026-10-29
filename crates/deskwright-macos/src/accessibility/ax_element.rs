use std::ptr;
use std::time::Duration;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use core_foundation::{declare_TCFType, impl_TCFType};
use deskwright_core::model::app::ProcessId;
use deskwright_core::model::geometry::Point;
use deskwright_core::model::native_element::NativeElement;
use deskwright_core::{DeskwrightError, DeskwrightResult};

use super::ax_error_code::AxErrorCode;
use super::ffi;

declare_TCFType!(AxUiElement, AxUiElementRef);
impl_TCFType!(AxUiElement, AxUiElementRef, AXUIElementGetTypeID);

use ffi::{AXUIElementGetTypeID, AxUiElementRef};

const NO_COPY_OPTIONS: u32 = 0;

/// A retained Accessibility element that can be shared across threads. This is the only
/// type that calls the raw element API; everything else goes through its safe methods.
#[derive(Clone)]
pub struct AxElement(AxUiElement);

// SAFETY: an AXUIElementRef is an immutable Core Foundation object. CFRetain/CFRelease are
// thread-safe, and the Accessibility API may be called from any thread because each request
// is an IPC message to the target app, so sharing or moving the handle is sound.
unsafe impl Send for AxElement {}
// SAFETY: see the `Send` implementation; no method mutates the handle itself.
unsafe impl Sync for AxElement {}

impl AxElement {
    pub fn application(process_id: ProcessId) -> DeskwrightResult<Self> {
        // SAFETY: AXUIElementCreateApplication has no preconditions and returns a +1 reference.
        let raw_element = unsafe { ffi::AXUIElementCreateApplication(process_id.0) };
        Self::from_created(raw_element, "open the app's Accessibility element")
    }

    pub fn system_wide() -> DeskwrightResult<Self> {
        // SAFETY: AXUIElementCreateSystemWide has no preconditions and returns a +1 reference.
        let raw_element = unsafe { ffi::AXUIElementCreateSystemWide() };
        Self::from_created(raw_element, "open the system-wide Accessibility element")
    }

    pub fn from_native(element: &NativeElement) -> DeskwrightResult<&Self> {
        element.downcast_ref::<Self>().ok_or_else(|| {
            DeskwrightError::operation_failed(
                "use element",
                "the handle was not created by the macOS adapter",
            )
        })
    }

    pub fn into_native(self) -> NativeElement {
        NativeElement::new(self)
    }

    /// Copies one attribute; `Ok(None)` when the element doesn't have it.
    pub fn attribute(&self, name: &str) -> Result<Option<CFType>, AxErrorCode> {
        let attribute_name = CFString::new(name);
        let mut value: CFTypeRef = ptr::null();
        // SAFETY: the element and attribute name are valid for the call; on success `value`
        // receives a +1 reference (copy rule) that we take ownership of below.
        let raw_error = unsafe {
            ffi::AXUIElementCopyAttributeValue(
                self.raw(),
                attribute_name.as_concrete_TypeRef(),
                &mut value,
            )
        };
        match AxErrorCode::check(raw_error) {
            Ok(()) if value.is_null() => Ok(None),
            // SAFETY: non-null +1 reference returned by a Copy function.
            Ok(()) => Ok(Some(unsafe { CFType::wrap_under_create_rule(value) })),
            Err(code) if code.means_value_absent() => Ok(None),
            Err(code) => Err(code),
        }
    }

    /// Copies several attributes in one IPC round trip. Missing attributes come back as
    /// error placeholders, which the value conversions treat as absent.
    pub fn attributes(&self, names: &[&str]) -> Result<Vec<CFType>, AxErrorCode> {
        let attribute_names: Vec<CFString> = names.iter().map(|name| CFString::new(name)).collect();
        let name_array = CFArray::from_CFTypes(&attribute_names);
        let mut values: CFArrayRef = ptr::null();
        // SAFETY: the element and name array are valid for the call; on success `values`
        // receives a +1 CFArray reference (copy rule).
        let raw_error = unsafe {
            ffi::AXUIElementCopyMultipleAttributeValues(
                self.raw(),
                name_array.as_concrete_TypeRef(),
                NO_COPY_OPTIONS,
                &mut values,
            )
        };
        AxErrorCode::check(raw_error)?;
        if values.is_null() {
            return Ok(Vec::new());
        }
        // SAFETY: non-null +1 CFArray of CF objects returned by a Copy function.
        let value_array: CFArray<CFType> = unsafe { CFArray::wrap_under_create_rule(values) };
        Ok(value_array.iter().map(|value| value.clone()).collect())
    }

    pub fn element(&self, name: &str) -> Result<Option<Self>, AxErrorCode> {
        Ok(self
            .attribute(name)?
            .and_then(|value| value.downcast_into::<AxUiElement>())
            .map(Self))
    }

    /// Reads an attribute holding a list of elements; empty when the attribute is absent.
    pub fn element_list(&self, name: &str) -> Result<Vec<Self>, AxErrorCode> {
        Ok(self
            .attribute(name)?
            .map(|value| Self::elements_in(&value))
            .unwrap_or_default())
    }

    /// The elements in an attribute value holding a list of elements; empty for any other
    /// value, such as the placeholder a batched request returns for a missing attribute.
    pub fn elements_in(value: &CFType) -> Vec<Self> {
        if value.type_of() != CFArray::<CFType>::type_id() {
            return Vec::new();
        }
        // SAFETY: the type check above guarantees `value` is a CFArray; the get rule retains
        // it independently of `value`, which outlives this call.
        let array: CFArray<CFType> =
            unsafe { CFArray::wrap_under_get_rule(value.as_CFTypeRef() as CFArrayRef) };
        array
            .iter()
            .filter_map(|item| item.downcast::<AxUiElement>())
            .map(Self)
            .collect()
    }

    pub fn set_attribute(&self, name: &str, value: &CFType) -> Result<(), AxErrorCode> {
        let attribute_name = CFString::new(name);
        // SAFETY: the element, name and value are valid CF objects for the duration of the call.
        let raw_error = unsafe {
            ffi::AXUIElementSetAttributeValue(
                self.raw(),
                attribute_name.as_concrete_TypeRef(),
                value.as_CFTypeRef(),
            )
        };
        AxErrorCode::check(raw_error)
    }

    pub fn perform_action(&self, name: &str) -> Result<(), AxErrorCode> {
        let action_name = CFString::new(name);
        // SAFETY: the element and action name are valid for the duration of the call.
        let raw_error =
            unsafe { ffi::AXUIElementPerformAction(self.raw(), action_name.as_concrete_TypeRef()) };
        AxErrorCode::check(raw_error)
    }

    pub fn process_id(&self) -> Result<ProcessId, AxErrorCode> {
        let mut raw_process_id = 0;
        // SAFETY: the element is valid and `raw_process_id` is a writable i32.
        let raw_error = unsafe { ffi::AXUIElementGetPid(self.raw(), &mut raw_process_id) };
        AxErrorCode::check(raw_error)?;
        Ok(ProcessId(raw_process_id))
    }

    /// The topmost element at a global screen point, asked of the system-wide element.
    pub fn element_at_position(&self, point: Point) -> Result<Option<Self>, AxErrorCode> {
        let mut raw_element: AxUiElementRef = ptr::null();
        // SAFETY: the element is valid and `raw_element` is writable; on success it receives
        // a +1 reference (copy rule) that we take ownership of below. Screen coordinates fit
        // comfortably in f32, which is what the API takes.
        let raw_error = unsafe {
            ffi::AXUIElementCopyElementAtPosition(
                self.raw(),
                point.x as f32,
                point.y as f32,
                &mut raw_element,
            )
        };
        match AxErrorCode::check(raw_error) {
            Ok(()) if raw_element.is_null() => Ok(None),
            // SAFETY: non-null +1 reference returned by a Copy function.
            Ok(()) => Ok(Some(Self(unsafe {
                AxUiElement::wrap_under_create_rule(raw_element)
            }))),
            Err(code) if code.means_nothing_at_position() => Ok(None),
            Err(code) => Err(code),
        }
    }

    pub fn set_messaging_timeout(&self, timeout: Duration) -> Result<(), AxErrorCode> {
        // SAFETY: the element is valid; any non-negative timeout is accepted.
        let raw_error =
            unsafe { ffi::AXUIElementSetMessagingTimeout(self.raw(), timeout.as_secs_f32()) };
        AxErrorCode::check(raw_error)
    }

    fn from_created(
        raw_element: AxUiElementRef,
        operation: &'static str,
    ) -> DeskwrightResult<Self> {
        if raw_element.is_null() {
            return Err(DeskwrightError::operation_failed(
                operation,
                "the system returned no element",
            ));
        }
        // SAFETY: non-null +1 reference from a Create function; ownership moves to the wrapper.
        Ok(Self(unsafe {
            AxUiElement::wrap_under_create_rule(raw_element)
        }))
    }

    fn raw(&self) -> AxUiElementRef {
        self.0.as_concrete_TypeRef()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn this_process_id() -> ProcessId {
        ProcessId(i32::try_from(std::process::id()).expect("process IDs fit in i32"))
    }

    #[test]
    fn creates_an_element_for_an_app_and_its_process_id_round_trips() {
        let app = AxElement::application(this_process_id()).unwrap();

        assert_eq!(app.process_id(), Ok(this_process_id()));
    }

    #[test]
    fn native_handles_round_trip_through_the_core() {
        let native = AxElement::system_wide().unwrap().into_native();

        assert!(AxElement::from_native(&native).is_ok());
    }

    #[test]
    fn finds_some_element_or_nothing_at_a_screen_point_without_failing() {
        let system_wide = AxElement::system_wide().unwrap();

        let hit = system_wide.element_at_position(Point { x: 1.0, y: 1.0 });

        // Without Accessibility permission the API reports it as disabled; with it, the menu
        // bar or desktop is found. Either way the call itself must be well-formed.
        if let Ok(Some(element)) = hit {
            assert!(element.process_id().is_ok());
        }
    }

    #[test]
    fn rejects_handles_created_by_other_adapters() {
        let foreign = NativeElement::new("not an accessibility element");

        let error = AxElement::from_native(&foreign).err().unwrap();

        assert!(
            error
                .to_string()
                .contains("not created by the macOS adapter")
        );
    }
}
