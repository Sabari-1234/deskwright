//! Converts Core Foundation attribute values into plain Rust values.

use std::ffi::c_void;

use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use deskwright_core::model::geometry::{Bounds, Point, Size};

use super::ffi::{self, AxValueType, CgPoint, CgSize};

pub fn to_text(value: &CFType) -> Option<String> {
    if let Some(text) = value.downcast::<CFString>() {
        return Some(text.to_string()).filter(|text| !text.is_empty());
    }
    if let Some(number) = value.downcast::<CFNumber>() {
        return number.to_f64().map(format_number);
    }
    value
        .downcast::<CFBoolean>()
        .map(|flag| bool::from(flag).to_string())
}

pub fn to_bool(value: &CFType) -> Option<bool> {
    value.downcast::<CFBoolean>().map(bool::from)
}

pub fn to_bounds(position: &CFType, size: &CFType) -> Option<Bounds> {
    let origin = read_ax_value::<CgPoint>(position, ffi::AX_VALUE_TYPE_CG_POINT)?;
    let extent = read_ax_value::<CgSize>(size, ffi::AX_VALUE_TYPE_CG_SIZE)?;
    Some(Bounds {
        x: origin.x,
        y: origin.y,
        width: extent.width,
        height: extent.height,
    })
}

pub fn from_point(point: Point) -> CFType {
    create_ax_value(
        ffi::AX_VALUE_TYPE_CG_POINT,
        &CgPoint {
            x: point.x,
            y: point.y,
        },
    )
}

pub fn from_size(size: Size) -> CFType {
    create_ax_value(
        ffi::AX_VALUE_TYPE_CG_SIZE,
        &CgSize {
            width: size.width,
            height: size.height,
        },
    )
}

/// Wraps a `#[repr(C)]` CGPoint/CGSize in an `AXValue` of `value_type`.
fn create_ax_value<Wrapped>(value_type: AxValueType, wrapped: &Wrapped) -> CFType {
    // SAFETY: `wrapped` has the C layout of `value_type`; AXValueCreate copies it and returns
    // a +1 reference that `wrap_under_create_rule` takes ownership of.
    unsafe {
        let raw_value =
            ffi::AXValueCreate(value_type, (wrapped as *const Wrapped).cast::<c_void>());
        CFType::wrap_under_create_rule(raw_value)
    }
}

fn format_number(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}

/// Reads a struct wrapped in an `AXValue`, if `value` is an `AXValue` of `expected_type`.
fn read_ax_value<Wrapped: Default>(value: &CFType, expected_type: AxValueType) -> Option<Wrapped> {
    // SAFETY: AXValueGetTypeID has no preconditions.
    if value.type_of() != unsafe { ffi::AXValueGetTypeID() } {
        return None;
    }
    // SAFETY: `value` is a valid AXValue (checked above).
    if unsafe { ffi::AXValueGetType(value.as_CFTypeRef()) } != expected_type {
        return None;
    }
    let mut wrapped = Wrapped::default();
    // SAFETY: the AXValue's type matches `expected_type`, whose C layout `Wrapped` mirrors
    // (`#[repr(C)]` CGPoint/CGSize), so writing into `wrapped` stays in bounds.
    let is_copied = unsafe {
        ffi::AXValueGetValue(
            value.as_CFTypeRef(),
            expected_type,
            (&mut wrapped as *mut Wrapped).cast::<c_void>(),
        )
    };
    (is_copied != 0).then_some(wrapped)
}

#[cfg(test)]
pub(crate) mod test_values {
    //! Builders for AXValue test inputs.

    use core_foundation::base::CFType;
    use deskwright_core::model::geometry::{Point, Size};

    pub fn point(x: f64, y: f64) -> CFType {
        super::from_point(Point { x, y })
    }

    pub fn size(width: f64, height: f64) -> CFType {
        super::from_size(Size { width, height })
    }
}

#[cfg(test)]
mod tests {
    use core_foundation::array::CFArray;

    use super::test_values::{point, size};
    use super::*;

    #[test]
    fn reads_strings_and_treats_empty_ones_as_absent() {
        assert_eq!(
            to_text(&CFString::new("Save").as_CFType()),
            Some("Save".to_owned())
        );
        assert_eq!(to_text(&CFString::new("").as_CFType()), None);
    }

    #[test]
    fn formats_whole_numbers_without_decimals() {
        assert_eq!(
            to_text(&CFNumber::from(42).as_CFType()),
            Some("42".to_owned())
        );
        assert_eq!(
            to_text(&CFNumber::from(0.5).as_CFType()),
            Some("0.5".to_owned())
        );
    }

    #[test]
    fn reads_booleans_as_text_and_as_flags() {
        let true_value = CFBoolean::true_value().as_CFType();

        assert_eq!(to_text(&true_value), Some("true".to_owned()));
        assert_eq!(to_bool(&true_value), Some(true));
        assert_eq!(to_bool(&CFString::new("true").as_CFType()), None);
    }

    #[test]
    fn ignores_values_of_other_types() {
        let empty_array = CFArray::<CFType>::from_CFTypes(&[]).as_CFType();

        assert_eq!(to_text(&empty_array), None);
    }

    #[test]
    fn combines_position_and_size_into_bounds() {
        let bounds = to_bounds(&point(10.0, 20.0), &size(30.0, 40.0));

        assert_eq!(
            bounds,
            Some(Bounds {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0
            })
        );
    }

    #[test]
    fn rejects_bounds_from_values_of_the_wrong_kind() {
        assert_eq!(to_bounds(&size(1.0, 1.0), &size(1.0, 1.0)), None);
        assert_eq!(
            to_bounds(&CFString::new("x").as_CFType(), &size(1.0, 1.0)),
            None
        );
    }
}
