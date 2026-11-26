use janus::StringHash;

use crate::env::{EnvValue, UiEnv};

pub trait PropertyValue: Clone + Default + Into<EnvValue> + From<EnvValue> {}
impl<T> PropertyValue for T where T: Clone + Default + Into<EnvValue> + From<EnvValue> {}

#[derive(Clone, Default, Debug)]
pub struct Property<T: PropertyValue, const ID: u64>(T);
impl<const ID: u64, T: PropertyValue + Copy> Copy for Property<T, ID> {}
impl<const ID: u64, T: PropertyValue> Property<T, ID> {
    pub fn new() -> Self {
        Self(T::default())
    }

    pub fn with_value(value: impl Into<T>) -> Self {
        Self(value.into())
    }

    pub const fn new_const(value: T) -> Self {
        Self(value)
    }

    pub fn sync_fetch(&mut self, env: &UiEnv) {
        let v = env.get(&Self::name()).cloned();
        self.0 = v.map(EnvValue::into).unwrap_or_default();
    }

    pub fn sync_push(&self, env: &mut UiEnv) {
        env.insert(Self::name(), self.0.clone());
    }

    pub const fn name() -> StringHash {
        StringHash::from_u64(ID)
    }

    pub fn set(&mut self, value: impl Into<T>) {
        self.0 = value.into();
    }

    pub const fn get(&self) -> &T {
        &self.0
    }

    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.0
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
