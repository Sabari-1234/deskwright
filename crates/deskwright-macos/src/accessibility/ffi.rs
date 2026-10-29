//! Raw declarations for the parts of the ApplicationServices Accessibility C API we use.
//! Only `ax_element` and `ax_value_conversion` call these, each with a `SAFETY:` argument.

use std::ffi::c_void;

use core_foundation::array::CFArrayRef;
use core_foundation::base::{Boolean, CFTypeID, CFTypeRef};
use core_foundation::string::CFStringRef;

#[repr(C)]
pub struct OpaqueAxUiElement {
    _private: [u8; 0],
}

pub type AxUiElementRef = *const OpaqueAxUiElement;
pub type AxValueRef = CFTypeRef;
pub type RawAxError = i32;
pub type AxValueType = u32;

pub const AX_VALUE_TYPE_CG_POINT: AxValueType = 1;
pub const AX_VALUE_TYPE_CG_SIZE: AxValueType = 2;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CgPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CgSize {
    pub width: f64,
    pub height: f64,
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn AXIsProcessTrusted() -> Boolean;
    pub fn AXUIElementGetTypeID() -> CFTypeID;
    pub fn AXUIElementCreateApplication(pid: i32) -> AxUiElementRef;
    pub fn AXUIElementCreateSystemWide() -> AxUiElementRef;
    pub fn AXUIElementGetPid(element: AxUiElementRef, pid: *mut i32) -> RawAxError;
    pub fn AXUIElementCopyAttributeValue(
        element: AxUiElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> RawAxError;
    pub fn AXUIElementCopyMultipleAttributeValues(
        element: AxUiElementRef,
        attributes: CFArrayRef,
        options: u32,
        values: *mut CFArrayRef,
    ) -> RawAxError;
    pub fn AXUIElementSetAttributeValue(
        element: AxUiElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> RawAxError;
    pub fn AXUIElementPerformAction(element: AxUiElementRef, action: CFStringRef) -> RawAxError;
    pub fn AXUIElementCopyElementAtPosition(
        application: AxUiElementRef,
        x: f32,
        y: f32,
        element: *mut AxUiElementRef,
    ) -> RawAxError;
    pub fn AXUIElementSetMessagingTimeout(
        element: AxUiElementRef,
        timeout_seconds: f32,
    ) -> RawAxError;
    pub fn AXValueCreate(value_type: AxValueType, value_ptr: *const c_void) -> AxValueRef;
    pub fn AXValueGetTypeID() -> CFTypeID;
    pub fn AXValueGetType(value: AxValueRef) -> AxValueType;
    pub fn AXValueGetValue(
        value: AxValueRef,
        value_type: AxValueType,
        value_ptr: *mut c_void,
    ) -> Boolean;
}
