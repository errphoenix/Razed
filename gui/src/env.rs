use std::fmt::Write;

use ethel::assets::CachedStringHash;
use janus::StringHash;

#[derive(Debug, Default)]
pub struct UiEnv {
    map: janus::StringMap<EnvValue>,
}
impl UiEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from<const N: usize>(values: [(StringHash, EnvValue); N]) -> Self {
        let mut map = janus::StringMap::with_capacity_and_hasher(values.len(), Default::default());

        for (hash, value) in values {
            map.insert(hash, value);
        }

        Self { map }
    }

    pub fn insert(&mut self, id: StringHash, value: impl Into<EnvValue>) -> Option<EnvValue> {
        self.map.insert(id, value.into())
    }

    pub fn remove(&mut self, id: &StringHash) -> Option<EnvValue> {
        self.map.remove(id)
    }

    pub fn get(&self, id: &StringHash) -> Option<&EnvValue> {
        self.map.get(id)
    }

    pub fn get_mut(&mut self, id: &StringHash) -> Option<&mut EnvValue> {
        self.map.get_mut(id)
    }
}

#[derive(Default, Clone, Debug, PartialEq, PartialOrd)]
pub enum EnvValue {
    #[default]
    Null,
    Boolean(bool),
    Integer(i32),
    Float(f32),
    HashedLiteral(CachedStringHash),
    DynamicString(String),
}
impl EnvValue {
    pub fn from_str(value: &'static str) -> Self {
        Self::HashedLiteral(ethel::assets::strings::hash(value))
    }

    pub fn from_string(value: String) -> Self {
        Self::DynamicString(value)
    }

    pub const fn is_null(&self) -> bool {
        matches!(self, EnvValue::Null)
    }

    pub const fn as_boolean(&self) -> Option<bool> {
        match self {
            EnvValue::Boolean(boolean) => Some(*boolean),
            _ => None,
        }
    }

    pub const fn as_integer(&self) -> Option<i32> {
        match self {
            EnvValue::Integer(integer) => Some(*integer),
            _ => None,
        }
    }

    pub const fn as_float(&self) -> Option<f32> {
        match self {
            EnvValue::Float(float) => Some(*float),
            _ => None,
        }
    }

    pub const fn as_hashed_literal(&self) -> Option<CachedStringHash> {
        match self {
            EnvValue::HashedLiteral(hash) => Some(*hash),
            _ => None,
        }
    }

    pub fn resolve_hashed_literal(&self) -> Option<&'static str> {
        self.as_hashed_literal().map(ethel::assets::strings::fetch)
    }

    pub const fn as_dynamic_string(&self) -> Option<&String> {
        match self {
            EnvValue::DynamicString(hash) => Some(hash),
            _ => None,
        }
    }

    pub const fn as_boolean_mut(&mut self) -> Option<&mut bool> {
        match self {
            EnvValue::Boolean(boolean) => Some(boolean),
            _ => None,
        }
    }

    pub const fn as_integer_mut(&mut self) -> Option<&mut i32> {
        match self {
            EnvValue::Integer(integer) => Some(integer),
            _ => None,
        }
    }

    pub const fn as_float_mut(&mut self) -> Option<&mut f32> {
        match self {
            EnvValue::Float(float) => Some(float),
            _ => None,
        }
    }

    pub const fn as_hashed_literal_mut(&mut self) -> Option<&mut CachedStringHash> {
        match self {
            EnvValue::HashedLiteral(hash) => Some(hash),
            _ => None,
        }
    }

    pub const fn as_dynamic_string_mut(&mut self) -> Option<&mut String> {
        match self {
            EnvValue::DynamicString(hash) => Some(hash),
            _ => None,
        }
    }

    pub fn write(&self, string: &mut String) -> std::fmt::Result {
        match self {
            EnvValue::Null => write!(string, "null"),
            EnvValue::Boolean(boolean) => write!(string, "{boolean}"),
            EnvValue::Integer(int) => write!(string, "{int}"),
            EnvValue::Float(float) => write!(string, "{float}"),
            EnvValue::HashedLiteral(_) => {
                write!(string, "{}", self.resolve_hashed_literal().unwrap())
            }
            EnvValue::DynamicString(dyn_str) => write!(string, "{dyn_str}"),
        }
    }
}
impl From<String> for EnvValue {
    fn from(value: String) -> Self {
        Self::from_string(value)
    }
}
impl From<&'static str> for EnvValue {
    fn from(value: &'static str) -> Self {
        Self::from_str(value)
    }
}
impl From<()> for EnvValue {
    fn from(_: ()) -> Self {
        Self::Null
    }
}
impl From<bool> for EnvValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}
impl From<i32> for EnvValue {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}
impl From<u32> for EnvValue {
    fn from(value: u32) -> Self {
        Self::Integer(value as i32)
    }
}
impl From<i16> for EnvValue {
    fn from(value: i16) -> Self {
        Self::Integer(value as i32)
    }
}
impl From<u16> for EnvValue {
    fn from(value: u16) -> Self {
        Self::Integer(value as i32)
    }
}
impl From<isize> for EnvValue {
    fn from(value: isize) -> Self {
        Self::Integer(value as i32)
    }
}
impl From<usize> for EnvValue {
    fn from(value: usize) -> Self {
        Self::Integer(value as i32)
    }
}
impl From<f32> for EnvValue {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}
