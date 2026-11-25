use ethel::{Column, IndirectIndex, Resolution};
use janus::StringHash;

use crate::{
    CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InterfaceSystem, TextOptions,
    WidgetError, WidgetId,
    declare::{UiDropdown, UiNodeStack},
};

pub const MAX_ENTRIES: usize = 16;

ethel::table_spec! {
    struct InterfaceDropdown {
        selected: u32;
        entries: [&'static str; MAX_ENTRIES];
        sync_handle: StringHash;

        open: bool;
        display_label: WidgetId;
        labels: [Option<WidgetId>; MAX_ENTRIES];
    }
}

impl InterfaceSystem {
    /// Make a core element into a dropdown-menu element.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the checkbox data.
    /// Or returns `None` if the given core `root_id` is invalid.
    pub fn make_dropdown(
        &mut self,
        root_id: WidgetId,
        display_label_id: WidgetId,
        starting_value: u32,
        entries: [&'static str; MAX_ENTRIES],
        sync_handle: StringHash,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.data.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;
            let dropdown_element = (
                starting_value,
                entries,
                sync_handle,
                false,
                display_label_id,
                Default::default(),
            );
            let dropdown_id = self.data.dropdowns.insert(dropdown_element);
            self.data.commons.archetype[commons_id.as_index()] = ComponentKind::Dropdown {
                handle: dropdown_id,
                display_text_handle: display_label_id,
            };
            Ok(dropdown_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
        }
    }
}

pub trait HasDropdownEntries {
    fn discriminant(&self) -> u32;

    fn entries() -> [DropdownEntry; MAX_ENTRIES];

    fn default() -> u32 {
        0
    }
}
impl<T: HasDropdownEntries> HasDropdownEntries for T
where
    T: Default,
{
    fn discriminant(&self) -> u32 {
        self.discriminant()
    }

    fn entries() -> [DropdownEntry; MAX_ENTRIES] {
        T::entries()
    }

    fn default() -> u32 {
        <Self as Default>::default().discriminant()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DropdownEntry {
    pub name: &'static str,
    pub discriminant: u32,
}
impl DropdownEntry {
    pub const fn empty() -> Self {
        Self::new("", 0)
    }

    pub const fn new(name: &'static str, discriminant: u32) -> Self {
        Self { name, discriminant }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DropdownOptions {
    pub starting_selection: Option<u32>,
    pub entries: [DropdownEntry; MAX_ENTRIES],
    pub sync_handle: StringHash,
    pub font_name: &'static str,
    pub font_size: f32,
}
impl DropdownOptions {
    pub const fn new(sync_handle: StringHash) -> Self {
        Self {
            sync_handle,
            starting_selection: None,
            entries: [DropdownEntry::new("", 0); MAX_ENTRIES],
            font_name: TextOptions::DEFAULT_FONT,
            font_size: TextOptions::DEFAULT_FONT_SIZE,
        }
    }

    pub const fn with_entries(self, set_entries: &[DropdownEntry]) -> Self {
        let mut entries = self.entries;
        let mut i = 0;
        while i < entries.len() && i < set_entries.len() {
            let entry = set_entries[i];
            assert!(entry.discriminant < MAX_ENTRIES as u32);
            entries[entry.discriminant as usize] = set_entries[i];
            i += 1;
        }
        Self { entries, ..self }
    }

    pub const fn with_starting_selection(self, starting_selection: u32) -> Self {
        Self {
            starting_selection: Some(starting_selection),
            ..self
        }
    }

    pub const fn with_sync_handle(self, sync_handle: StringHash) -> Self {
        Self {
            sync_handle,
            ..self
        }
    }

    pub const fn with_font_family(self, font_name: &'static str) -> Self {
        Self { font_name, ..self }
    }

    pub const fn with_font_size(self, font_size: f32) -> Self {
        Self { font_size, ..self }
    }
}

#[derive(Debug)]
pub struct Dropdown;
impl HasOptions for Dropdown {
    type Options = DropdownOptions;

    fn complement_options(
        common: CommonWidgetOptions,
        this: Self::Options,
        _: Resolution,
    ) -> ElementOptions {
        ElementOptions::Dropdown(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Checkbox(Default::default())
    }
}

impl UiNodeStack<'_> {
    pub fn dropdown(&mut self, dropdown_op: impl FnOnce(UiDropdown) -> UiDropdown) {
        self.push(dropdown_op(UiDropdown::new(self.reference_resolution)));
    }

    pub fn dropdown_from(&mut self, dropdown_options: DropdownOptions) {
        self.push(UiDropdown::from(
            dropdown_options,
            self.reference_resolution,
        ));
    }

    pub fn dropdown_enum<T: HasDropdownEntries>(&mut self, sync_id: StringHash) {
        self.dropdown_from(
            DropdownOptions::new(sync_id)
                .with_entries(&T::entries())
                .with_starting_selection(T::default()),
        );
    }
}
