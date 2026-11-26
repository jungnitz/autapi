use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque},
    error::Error,
    fmt,
    rc::Rc,
    sync::Arc,
};

use crate::schema::Nullable;

/// A type whose values can be checked against the validation constraints of its schema.
///
/// This trait is implemented by the [`ToSchema`](crate::schema::ToSchema) derive macro, which
/// checks the constraints declared via `#[schema(...)]` attributes (e.g. `min_length`, `maximum`
/// or `pattern`). Values of nested types are not validated again, as they are validated when they
/// are deserialized themselves.
pub trait Validate {
    /// Checks whether this value satisfies all constraints, returning the first violation.
    fn validate(&self) -> Result<(), ValidationError>;
}

/// A violation of a validation constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    property: Option<Cow<'static, str>>,
    kind: ValidationErrorKind,
}

impl ValidationError {
    pub fn new(kind: ValidationErrorKind) -> Self {
        Self {
            property: None,
            kind,
        }
    }
    /// Sets the name of the property whose value violates the constraint.
    pub fn with_property(self, property: impl Into<Cow<'static, str>>) -> Self {
        Self {
            property: Some(property.into()),
            ..self
        }
    }
    /// Returns the name of the property whose value violates the constraint, if any.
    pub fn property(&self) -> Option<&str> {
        self.property.as_deref()
    }
    /// Returns the violated constraint.
    pub fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.property {
            Some(property) => write!(f, "`{property}`: {}", self.kind),
            None => self.kind.fmt(f),
        }
    }
}

impl Error for ValidationError {}

/// The constraint violated by a value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ValidationErrorKind {
    Minimum(Numeric),
    Maximum(Numeric),
    ExclusiveMinimum(Numeric),
    ExclusiveMaximum(Numeric),
    MultipleOf(Numeric),
    MinLength(u64),
    MaxLength(u64),
    MinItems(u64),
    MaxItems(u64),
    MinProperties(u64),
    MaxProperties(u64),
    Pattern(Cow<'static, str>),
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minimum(n) => write!(f, "must be at least {n}"),
            Self::Maximum(n) => write!(f, "must be at most {n}"),
            Self::ExclusiveMinimum(n) => write!(f, "must be greater than {n}"),
            Self::ExclusiveMaximum(n) => write!(f, "must be less than {n}"),
            Self::MultipleOf(n) => write!(f, "must be a multiple of {n}"),
            Self::MinLength(n) => write!(f, "must be at least {n} characters long"),
            Self::MaxLength(n) => write!(f, "must be at most {n} characters long"),
            Self::MinItems(n) => write!(f, "must contain at least {n} items"),
            Self::MaxItems(n) => write!(f, "must contain at most {n} items"),
            Self::MinProperties(n) => write!(f, "must contain at least {n} properties"),
            Self::MaxProperties(n) => write!(f, "must contain at most {n} properties"),
            Self::Pattern(pattern) => write!(f, "must match the pattern `{pattern}`"),
        }
    }
}

/// A number that can be compared with numbers of other primitive types without loss of precision
/// (except for comparisons with floats).
#[derive(Debug, Clone, Copy)]
pub enum Numeric {
    Int(i128),
    UInt(u128),
    Float(f64),
}

impl Numeric {
    fn as_f64(self) -> f64 {
        match self {
            Self::Int(n) => n as f64,
            Self::UInt(n) => n as f64,
            Self::Float(n) => n,
        }
    }
    fn as_int_magnitude(self) -> Option<u128> {
        match self {
            Self::Int(n) => Some(n.unsigned_abs()),
            Self::UInt(n) => Some(n),
            Self::Float(_) => None,
        }
    }
    /// Returns whether this number is an integer multiple of `divisor`.
    pub fn is_multiple_of(self, divisor: Numeric) -> bool {
        if let (Some(n), Some(d)) = (self.as_int_magnitude(), divisor.as_int_magnitude()) {
            return d != 0 && n % d == 0;
        }
        let quotient = self.as_f64() / divisor.as_f64();
        quotient.is_finite() && (quotient - quotient.round()).abs() < 1e-9
    }
}

impl PartialEq for Numeric {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl PartialOrd for Numeric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (Self::Int(a), Self::Int(b)) => a.partial_cmp(&b),
            (Self::UInt(a), Self::UInt(b)) => a.partial_cmp(&b),
            (Self::Int(a), Self::UInt(b)) => Some(match u128::try_from(a) {
                Ok(a) => a.cmp(&b),
                Err(_) => Ordering::Less,
            }),
            (Self::UInt(_), Self::Int(_)) => other.partial_cmp(self).map(Ordering::reverse),
            (a, b) => a.as_f64().partial_cmp(&b.as_f64()),
        }
    }
}

impl fmt::Display for Numeric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => n.fmt(f),
            Self::UInt(n) => n.fmt(f),
            Self::Float(n) => n.fmt(f),
        }
    }
}

/// A type that can be validated with `minimum`, `maximum`, `exclusive_minimum`,
/// `exclusive_maximum` and `multiple_of`.
///
/// Returning `None` skips validation (e.g. for absent optional values).
pub trait ValidateNumber {
    fn validation_number(&self) -> Option<Numeric>;
}

/// A type that can be validated with `min_length`, `max_length` and `pattern`.
///
/// Returning `None` skips validation (e.g. for absent optional values).
pub trait ValidateStr {
    fn validation_str(&self) -> Option<&str>;
}

/// A type that can be validated with `min_items` and `max_items`.
///
/// Returning `None` skips validation (e.g. for absent optional values).
pub trait ValidateItems {
    fn validation_items(&self) -> Option<usize>;
}

/// A type that can be validated with `min_properties` and `max_properties`.
///
/// Returning `None` skips validation (e.g. for absent optional values).
pub trait ValidateProperties {
    fn validation_properties(&self) -> Option<usize>;
}

macro_rules! impl_validate_number {
    ($variant:ident: $($ty:ty),*) => {
        $(
            impl ValidateNumber for $ty {
                fn validation_number(&self) -> Option<Numeric> {
                    Some(Numeric::$variant((*self).into()))
                }
            }
        )*
    };
}

impl_validate_number!(Int: i8, i16, i32, i64, i128);
impl_validate_number!(UInt: u8, u16, u32, u64, u128);
impl_validate_number!(Float: f32, f64);

impl ValidateNumber for isize {
    fn validation_number(&self) -> Option<Numeric> {
        Some(Numeric::Int(*self as i128))
    }
}
impl ValidateNumber for usize {
    fn validation_number(&self) -> Option<Numeric> {
        Some(Numeric::UInt(*self as u128))
    }
}

impl ValidateStr for str {
    fn validation_str(&self) -> Option<&str> {
        Some(self)
    }
}

impl ValidateStr for String {
    fn validation_str(&self) -> Option<&str> {
        Some(self)
    }
}

macro_rules! impl_validate_len {
    ($trait:ident::$method:ident for $($ty:ident<$($gen:ident),*>),*) => {
        $(
            impl<$($gen),*> $trait for $ty<$($gen),*> {
                fn $method(&self) -> Option<usize> {
                    Some(self.len())
                }
            }
        )*
    };
}

impl_validate_len!(
    ValidateItems::validation_items
    for Vec<T>, VecDeque<T>, LinkedList<T>, HashSet<T, S>, BTreeSet<T>
);
impl_validate_len!(
    ValidateProperties::validation_properties
    for HashMap<K, V, S>, BTreeMap<K, V>
);

impl<T> ValidateItems for [T] {
    fn validation_items(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<T, const N: usize> ValidateItems for [T; N] {
    fn validation_items(&self) -> Option<usize> {
        Some(N)
    }
}

/// Forwards a validation trait through wrapper types.
macro_rules! forward_validate {
    ($trait:ident::$method:ident -> $ret:ty) => {
        impl<T: $trait + ?Sized> $trait for &T {
            fn $method(&self) -> $ret {
                (**self).$method()
            }
        }
        impl<T: $trait + ?Sized> $trait for Box<T> {
            fn $method(&self) -> $ret {
                (**self).$method()
            }
        }
        impl<T: $trait + ?Sized> $trait for Arc<T> {
            fn $method(&self) -> $ret {
                (**self).$method()
            }
        }
        impl<T: $trait + ?Sized> $trait for Rc<T> {
            fn $method(&self) -> $ret {
                (**self).$method()
            }
        }
        impl<T: $trait + ToOwned + ?Sized> $trait for Cow<'_, T> {
            fn $method(&self) -> $ret {
                (**self).$method()
            }
        }
        impl<T: $trait> $trait for Option<T> {
            fn $method(&self) -> $ret {
                self.as_ref().and_then(T::$method)
            }
        }
        impl<T: $trait> $trait for Nullable<T> {
            fn $method(&self) -> $ret {
                self.0.$method()
            }
        }
    };
}

forward_validate!(ValidateNumber::validation_number -> Option<Numeric>);
forward_validate!(ValidateStr::validation_str -> Option<&str>);
forward_validate!(ValidateItems::validation_items -> Option<usize>);
forward_validate!(ValidateProperties::validation_properties -> Option<usize>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_comparisons() {
        assert!(Numeric::Int(-1) < Numeric::UInt(0));
        assert!(Numeric::UInt(u128::MAX) > Numeric::Int(i128::MAX));
        assert!(Numeric::Int(3) == Numeric::UInt(3));
        assert!(Numeric::Float(2.5) > Numeric::Int(2));
        assert!(
            Numeric::Float(f64::NAN)
                .partial_cmp(&Numeric::Int(0))
                .is_none()
        );
    }

    #[test]
    fn numeric_multiple_of() {
        assert!(Numeric::Int(-9).is_multiple_of(Numeric::UInt(3)));
        assert!(!Numeric::Int(10).is_multiple_of(Numeric::Int(3)));
        assert!(Numeric::Float(0.3).is_multiple_of(Numeric::Float(0.1)));
        assert!(!Numeric::Float(0.35).is_multiple_of(Numeric::Float(0.1)));
        assert!(!Numeric::Int(1).is_multiple_of(Numeric::Int(0)));
    }
}
