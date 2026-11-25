use ethel::{Column, IndirectIndex, Resolution};
use janus::StringHash;

use crate::{
    ButtonCallback, CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions,
    InterfaceSystem, TextOptions, WidgetError, WidgetId,
    declare::{UiDropdown, UiNodeStack, UiPanel},
};

pub const MAX_ENTRIES: usize = 16;

ethel::table_spec! {
    struct InterfaceDropdown {
        selected: u32;
        entries: [&'static str; MAX_ENTRIES];
        sync_handle: StringHash;

        open: bool;
        display_sync_handle: StringHash;
        display_label: WidgetId;
        extended_panel: WidgetId;
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
        extended_panel_id: WidgetId,
        display_sync_handle: StringHash,
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
                display_sync_handle,
                display_label_id,
                extended_panel_id,
            );
            let dropdown_id = self.data.dropdowns.insert(dropdown_element);
            self.data.commons.archetype[commons_id.as_index()] = ComponentKind::Dropdown {
                handle: dropdown_id,
                display_text_handle: display_label_id,
                extended_menu: extended_panel_id,
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

    pub const fn is_empty(&self) -> bool {
        self.name.is_empty() && self.discriminant == 0
    }

    pub const fn new(name: &'static str, discriminant: u32) -> Self {
        Self { name, discriminant }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DropdownOptions {
    pub starting_selection: Option<u32>,
    pub entries: [DropdownEntry; MAX_ENTRIES],
    pub sync_handle: StringHash,
    pub font_name: &'static str,
    pub font_size: f32,
    pub extended_options_y_margin: Option<f32>,
}
impl Default for DropdownOptions {
    fn default() -> Self {
        Self::new(StringHash::from_u64(0))
    }
}
impl DropdownOptions {
    pub const fn new(sync_handle: StringHash) -> Self {
        Self {
            sync_handle,
            starting_selection: None,
            entries: [DropdownEntry::new("", 0); MAX_ENTRIES],
            font_name: TextOptions::DEFAULT_FONT,
            font_size: TextOptions::DEFAULT_FONT_SIZE,
            extended_options_y_margin: None,
        }
    }

    /// The vertical margin between options in the extended dropdown menu.
    pub const fn with_options_y_margin(self, margin: f32) -> Self {
        Self {
            extended_options_y_margin: Some(margin),
            ..self
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

pub fn extended_menu_panel(
    root_id: WidgetId,
    reference_resolution: Resolution,
    layer: u32,
    dropdown_options: &DropdownOptions,
) -> UiPanel {
    let mut panel = UiPanel::new(reference_resolution)
        .with_parent(root_id)
        .with_layer(layer)
        .with_visible(false)
        .builder(|mut stack, props| {
            for entry in dropdown_options.entries {
                if !entry.is_empty() {
                    stack.button(|btn| {
                        btn.builder(|_, mut props| {
                            props = props.with_label_literal(entry.name).with_callback(
                                ButtonCallback::OnceWithData {
                                    op: |env, _, buf| {
                                        let menu_curr_option = buf[0];
                                        let option_discriminant = buf[1].as_u64();
                                        env.insert(menu_curr_option, option_discriminant);
                                    },
                                    data: (&[
                                        dropdown_options.sync_handle,
                                        StringHash::from_u64(entry.discriminant as u64),
                                    ])
                                        .into(),
                                },
                            );

                            props
                        })
                    });
                }
            }
            props
        });

    if let Some(y_margin) = dropdown_options.extended_options_y_margin {
        panel = panel.with_margin((0.0, y_margin, 0.0, y_margin));
    }

    panel
}

pub fn encode_dropdown_id_display_handle(
    root_id: WidgetId,
    options: &DropdownOptions,
) -> StringHash {
    let mut str = String::from("@dropdown/options=[");
    for (i, entry) in options.entries.iter().filter(|e| !e.is_empty()).enumerate() {
        if i > 0 {
            str.push_str(",");
        }
        str.push_str(entry.name);
    }
    str.push_str("]-ID=");
    str.push_str(&format!("{}", root_id.0.as_index()));
    str.push_str("/display");
    janus::hash_string(&str)
}
