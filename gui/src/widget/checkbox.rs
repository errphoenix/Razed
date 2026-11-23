use ethel::{Column, IndirectIndex, Resolution};
use janus::StringHash;

use crate::{
    CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InterfaceSystem, Point,
    Property, WidgetError, WidgetId,
    declare::{UiCheckbox, UiNodeStack},
};

ethel::table_spec! {
    struct InterfaceCheckbox {
        value: bool;
        sync_handle: Option<StringHash>;
    }
}

impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    /// Make a core element into a checkbox element.
    ///
    /// If a label is present, this also requires the `text_id` of a text
    /// element, created with [`Self::create_text`].
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the checkbox data.
    /// Or returns `None` if the given core `root_id` is invalid.
    pub fn make_checkbox(
        &mut self,
        root_id: WidgetId,
        starting_value: bool,
        sync_handle: Option<StringHash>,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;
            let checkbox_element = (starting_value, sync_handle);
            let checkbox_id = self.checkboxes.insert(checkbox_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Checkbox(checkbox_id);
            Ok(checkbox_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
        }
    }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct CheckboxOptions {
    pub starting_value: bool,
    pub sync_handle: Option<StringHash>,
    pub size: f32,
}
impl CheckboxOptions {
    pub const DEFAULT_SIZE: f32 = 22.0;

    pub const fn new() -> Self {
        Self {
            starting_value: false,
            sync_handle: None,
            size: Self::DEFAULT_SIZE,
        }
    }

    pub const fn value(value: bool) -> Self {
        Self::new().with_value(value)
    }

    pub const fn synced(handle: StringHash) -> Self {
        Self::new().with_sync_handle(handle)
    }

    pub const fn with_size(self, size: f32) -> Self {
        Self { size, ..self }
    }

    pub const fn with_value(self, value: bool) -> Self {
        Self {
            starting_value: value,
            ..self
        }
    }

    pub const fn with_sync_handle(self, sync_handle: StringHash) -> Self {
        Self {
            sync_handle: Some(sync_handle),
            ..self
        }
    }
}

#[derive(Debug)]
pub struct Checkbox;
impl HasOptions for Checkbox {
    type Options = CheckboxOptions;

    fn complement_options(
        mut common: CommonWidgetOptions,
        this: Self::Options,
        _: Resolution,
    ) -> ElementOptions {
        let this_size = Point::splat_px(this.size);
        common.layout.size = Some(this_size);
        ElementOptions::Checkbox(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Checkbox(Default::default())
    }
}

impl UiNodeStack<'_> {
    pub fn checkbox(&mut self, checkbox_op: impl FnOnce(UiCheckbox) -> UiCheckbox) {
        self.push(checkbox_op(UiCheckbox::new(self.reference_resolution)));
    }

    pub fn checkbox_from(&mut self, checkbox_options: CheckboxOptions) {
        self.push(UiCheckbox::from(
            checkbox_options,
            self.reference_resolution,
        ));
    }

    pub fn checkbox_value(&mut self, checkbox_value: bool) {
        self.checkbox_from(CheckboxOptions::value(checkbox_value));
    }

    pub fn checkbox_synced(&mut self, handle: StringHash) {
        self.checkbox_from(CheckboxOptions::synced(handle));
    }

    pub fn checkbox_prop<const PROP_ID: u64>(&mut self, prop: Property<bool, PROP_ID>) {
        let name = Property::<bool, PROP_ID>::name();
        self.checkbox_from(CheckboxOptions::synced(name).with_value(*prop.get()));
    }
}
