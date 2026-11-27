use janus::StringHash;

use crate::env::{EnvValue, UiEnv};

pub trait PropertyValue: Clone + Default + Into<EnvValue> + From<EnvValue> {}
impl<T> PropertyValue for T where T: Clone + Default + Into<EnvValue> + From<EnvValue> {}

#[derive(Clone, Default, Debug)]
pub struct Property<T: PropertyValue, const ID: u64> {
    inner: T,
    // only store unmapped value if a
    // mapping function is defined
    inner_unmapped: Option<T>,
    map_fn: Option<fn(&mut T)>,
}
impl<const ID: u64, T: PropertyValue + Copy> Copy for Property<T, ID> {}
impl<const ID: u64, T: PropertyValue> Property<T, ID> {
    pub fn new() -> Self {
        Self {
            inner: T::default(),
            inner_unmapped: None,
            map_fn: None,
        }
    }

    pub fn new_mapped(map_fn: fn(&mut T)) -> Self {
        Self {
            inner: T::default(),
            inner_unmapped: None,
            map_fn: Some(map_fn),
        }
    }

    pub fn with_value(value: impl Into<T>) -> Self {
        Self {
            inner: value.into(),
            inner_unmapped: None,
            map_fn: None,
        }
    }

    pub fn with_value_mapped(value: impl Into<T>, map_fn: fn(&mut T)) -> Self {
        let unmapped = value.into();
        let mut mapped = unmapped.clone();
        map_fn(&mut mapped);
        Self {
            inner: mapped,
            inner_unmapped: Some(unmapped),
            map_fn: Some(map_fn),
        }
    }

    pub const fn new_const(value: T) -> Self {
        Self {
            inner: value,
            inner_unmapped: None,
            map_fn: None,
        }
    }

    pub fn sync_fetch(&mut self, env: &UiEnv) {
        let env_var = env.get(&Self::name()).cloned();
        let mut typed = env_var.map(EnvValue::into).unwrap_or_default();
        if let Some(map_fn) = self.map_fn {
            map_fn(&mut typed);
        }
        self.inner = typed;
    }

    pub fn sync_push(&self, env: &mut UiEnv) {
        if let Some(unmapped) = &self.inner_unmapped {
            env.insert(Self::name(), unmapped.clone());
        } else {
            env.insert(Self::name(), self.inner.clone());
        }
    }

    pub const fn name() -> StringHash {
        StringHash::from_u64(ID)
    }

    pub fn set(&mut self, value: impl Into<T>) {
        self.inner = value.into();
    }

    pub const fn get(&self) -> &T {
        &self.inner
    }

    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
impl<T: PropertyValue, const ID: u64> std::ops::Deref for Property<T, ID> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
impl<T: PropertyValue, const ID: u64> std::ops::DerefMut for Property<T, ID> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
