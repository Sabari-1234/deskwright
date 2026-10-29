use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use deskwright_core::model::app::ProcessId;
use deskwright_core::model::element::ElementAttributes;
use deskwright_core::model::native_element::NativeElement;
use deskwright_core::ports::ElementTreeReader;
use deskwright_core::ports::element_tree_reader::ElementContents;
use deskwright_core::{DeskwrightError, DeskwrightResult};

use super::ax_element::AxElement;
use super::ax_names;
use super::ax_value_conversion::{to_bool, to_bounds, to_text};

/// Attributes read for every element in one batched request (order matters), followed by its
/// children, so a tree walk needs a single IPC round trip per element.
const ELEMENT_AND_CHILDREN_ATTRIBUTE_NAMES: [&str; 11] = [
    ax_names::ROLE,
    ax_names::TITLE,
    ax_names::VALUE,
    ax_names::DESCRIPTION,
    ax_names::PLACEHOLDER,
    ax_names::IDENTIFIER,
    ax_names::ENABLED,
    ax_names::FOCUSED,
    ax_names::POSITION,
    ax_names::SIZE,
    ax_names::CHILDREN,
];
/// The leading entries above that describe the element itself.
const ELEMENT_ATTRIBUTE_COUNT: usize = 10;

const MENU_ROLE: &str = "AXMenu";

fn is_menu(element: &AxElement) -> bool {
    element
        .attribute(ax_names::ROLE)
        .ok()
        .flatten()
        .and_then(|role| to_text(&role))
        .is_some_and(|role| role == MENU_ROLE)
}

/// [`ElementTreeReader`] over the macOS Accessibility API.
#[derive(Debug, Default)]
pub struct AxTreeReader;

impl AxTreeReader {
    pub fn new() -> Self {
        Self
    }
}

impl ElementTreeReader for AxTreeReader {
    fn top_level_elements(&self, process_id: ProcessId) -> DeskwrightResult<Vec<NativeElement>> {
        let app = AxElement::application(process_id)?;
        expose_web_content(&app);
        let windows = app
            .element_list(ax_names::WINDOWS)
            .map_err(|code| code.into_deskwright_error("read the app's windows"))?;
        let children = app
            .element_list(ax_names::CHILDREN)
            .map_err(|code| code.into_deskwright_error("read the app's elements"))?;
        let top_level = if windows.is_empty() {
            children
        } else {
            // Open pop-up and context menus hang off the app, not their window or button, so
            // they are listed first (they are frontmost) or their items could not be found.
            let mut open_menus: Vec<AxElement> = children.into_iter().filter(is_menu).collect();
            open_menus.extend(windows);
            open_menus
        };
        Ok(top_level.into_iter().map(AxElement::into_native).collect())
    }

    /// One IPC round trip instead of two: every element of a tree walk goes through here.
    fn read_with_children(&self, element: &NativeElement) -> DeskwrightResult<ElementContents> {
        let mut values = AxElement::from_native(element)?
            .attributes(&ELEMENT_AND_CHILDREN_ATTRIBUTE_NAMES)
            .map_err(|code| code.into_deskwright_error("read element attributes"))?;
        let children = values
            .pop()
            .filter(|_| values.len() == ELEMENT_ATTRIBUTE_COUNT)
            .map(|children_value| AxElement::elements_in(&children_value))
            .unwrap_or_default();
        Ok(ElementContents {
            attributes: attributes_from_values(values)?,
            children: children.into_iter().map(AxElement::into_native).collect(),
        })
    }

    fn attributes(&self, element: &NativeElement) -> DeskwrightResult<ElementAttributes> {
        let values = AxElement::from_native(element)?
            .attributes(&ELEMENT_AND_CHILDREN_ATTRIBUTE_NAMES[..ELEMENT_ATTRIBUTE_COUNT])
            .map_err(|code| code.into_deskwright_error("read element attributes"))?;
        attributes_from_values(values)
    }

    fn children(&self, element: &NativeElement) -> DeskwrightResult<Vec<NativeElement>> {
        let children = AxElement::from_native(element)?
            .element_list(ax_names::CHILDREN)
            .map_err(|code| code.into_deskwright_error("read element children"))?;
        Ok(children.into_iter().map(AxElement::into_native).collect())
    }
}

/// Electron apps (VS Code, Slack…) only build the accessibility tree of their web content
/// once a client sets this attribute. Apps that don't support it reject it, so that error is
/// expected and deliberately ignored.
fn expose_web_content(app: &AxElement) {
    let _ = app.set_attribute(
        ax_names::MANUAL_ACCESSIBILITY,
        &CFBoolean::true_value().as_CFType(),
    );
}

fn attributes_from_values(values: Vec<CFType>) -> DeskwrightResult<ElementAttributes> {
    let value_count = values.len();
    let [
        role,
        title,
        value,
        description,
        placeholder,
        identifier,
        enabled,
        focused,
        position,
        size,
    ] = <[CFType; ELEMENT_ATTRIBUTE_COUNT]>::try_from(values).map_err(|_| {
        DeskwrightError::operation_failed(
            "read element attributes",
            format!(
                "expected {} values, got {value_count}",
                ELEMENT_ATTRIBUTE_COUNT
            ),
        )
    })?;
    Ok(ElementAttributes {
        role: to_text(&role).unwrap_or_default(),
        title: to_text(&title),
        value: to_text(&value),
        description: to_text(&description),
        placeholder: to_text(&placeholder),
        identifier: to_text(&identifier),
        // Windows, text areas and groups have no AXEnabled attribute at all; they are usable,
        // so only an explicit `false` marks an element as disabled.
        is_enabled: to_bool(&enabled).unwrap_or(true),
        is_focused: to_bool(&focused).unwrap_or(false),
        bounds: to_bounds(&position, &size),
    })
}

#[cfg(test)]
mod tests {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::CFString;
    use deskwright_core::model::geometry::Bounds;

    use super::super::ax_value_conversion::test_values::{point, size};
    use super::*;

    fn text(value: &str) -> CFType {
        CFString::new(value).as_CFType()
    }

    #[test]
    fn builds_attributes_from_the_batched_values_in_order() {
        let values = vec![
            text("AXButton"),
            text("Save"),
            text(""),
            text("Saves the file"),
            text(""),
            text("save-button"),
            CFBoolean::true_value().as_CFType(),
            CFBoolean::false_value().as_CFType(),
            point(1.0, 2.0),
            size(3.0, 4.0),
        ];

        let attributes = attributes_from_values(values).unwrap();

        assert_eq!(
            attributes,
            ElementAttributes {
                role: "AXButton".to_owned(),
                title: Some("Save".to_owned()),
                value: None,
                description: Some("Saves the file".to_owned()),
                placeholder: None,
                identifier: Some("save-button".to_owned()),
                is_enabled: true,
                is_focused: false,
                bounds: Some(Bounds {
                    x: 1.0,
                    y: 2.0,
                    width: 3.0,
                    height: 4.0
                }),
            }
        );
    }

    #[test]
    fn treats_elements_without_an_enabled_attribute_as_enabled() {
        let missing = text("");
        let mut values = vec![text("AXTextArea")];
        values.extend(std::iter::repeat_n(missing, ELEMENT_ATTRIBUTE_COUNT - 1));

        let attributes = attributes_from_values(values).unwrap();

        assert!(attributes.is_enabled);
        assert!(!attributes.is_focused);
    }

    #[test]
    fn reports_an_explicitly_disabled_element_as_disabled() {
        let mut values = vec![text("AXButton")];
        values.extend((0..5).map(|_| text("")));
        values.push(CFBoolean::false_value().as_CFType());
        values.extend((0..3).map(|_| text("")));

        assert!(!attributes_from_values(values).unwrap().is_enabled);
    }

    #[test]
    fn fails_clearly_when_the_api_returns_the_wrong_number_of_values() {
        let error = attributes_from_values(vec![text("AXButton")]).unwrap_err();

        assert!(error.to_string().contains("expected 10 values, got 1"));
    }
}
