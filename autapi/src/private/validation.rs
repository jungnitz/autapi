use crate::schema::{
    Numeric, ValidateItems, ValidateNumber, ValidateProperties, ValidateStr, ValidationError,
    ValidationErrorKind,
};

type Property = Option<&'static str>;

fn check(
    valid: bool,
    kind: ValidationErrorKind,
    property: Property,
) -> Result<(), ValidationError> {
    if valid {
        return Ok(());
    }
    let error = ValidationError::new(kind);
    Err(match property {
        Some(property) => error.with_property(property),
        None => error,
    })
}

macro_rules! number_check {
    ($name:ident, $kind:ident: |$value:ident, $bound:ident| $valid:expr) => {
        pub fn $name<T: ValidateNumber + ?Sized>(
            value: &T,
            $bound: Numeric,
            property: Property,
        ) -> Result<(), ValidationError> {
            match value.validation_number() {
                Some($value) => check($valid, ValidationErrorKind::$kind($bound), property),
                None => Ok(()),
            }
        }
    };
}

number_check!(minimum, Minimum: |value, bound| value >= bound);
number_check!(maximum, Maximum: |value, bound| value <= bound);
number_check!(exclusive_minimum, ExclusiveMinimum: |value, bound| value > bound);
number_check!(exclusive_maximum, ExclusiveMaximum: |value, bound| value < bound);
number_check!(multiple_of, MultipleOf: |value, bound| value.is_multiple_of(bound));

macro_rules! len_check {
    ($name:ident, $kind:ident, $trait:ident::$method:ident: |$len:ident, $bound:ident| $valid:expr) => {
        pub fn $name<T: $trait + ?Sized>(
            value: &T,
            $bound: u64,
            property: Property,
        ) -> Result<(), ValidationError> {
            match value.$method() {
                Some($len) => check($valid, ValidationErrorKind::$kind($bound), property),
                None => Ok(()),
            }
        }
    };
}

len_check!(min_items, MinItems, ValidateItems::validation_items: |len, bound| len as u64 >= bound);
len_check!(max_items, MaxItems, ValidateItems::validation_items: |len, bound| len as u64 <= bound);
len_check!(min_properties, MinProperties, ValidateProperties::validation_properties: |len, bound| len as u64 >= bound);
len_check!(max_properties, MaxProperties, ValidateProperties::validation_properties: |len, bound| len as u64 <= bound);

/// Checks `min_length` and `max_length` together, counting the characters only once.
pub fn length<T: ValidateStr + ?Sized>(
    value: &T,
    min: Option<u64>,
    max: Option<u64>,
    property: Property,
) -> Result<(), ValidationError> {
    let Some(s) = value.validation_str() else {
        return Ok(());
    };
    // there is no need to count beyond the first character exceeding `max`
    let limit = match max {
        Some(max) => usize::try_from(max).map_or(usize::MAX, |max| max.saturating_add(1)),
        None => usize::MAX,
    };
    let len = s.chars().take(limit).count() as u64;
    if let Some(min) = min {
        check(len >= min, ValidationErrorKind::MinLength(min), property)?;
    }
    if let Some(max) = max {
        check(len <= max, ValidationErrorKind::MaxLength(max), property)?;
    }
    Ok(())
}

#[cfg(feature = "regex")]
pub fn pattern<T: ValidateStr + ?Sized>(
    value: &T,
    regex: &regex::Regex,
    pattern: &'static str,
    property: Property,
) -> Result<(), ValidationError> {
    match value.validation_str() {
        Some(s) => check(
            regex.is_match(s),
            ValidationErrorKind::Pattern(pattern.into()),
            property,
        ),
        None => Ok(()),
    }
}

/// Validates a value against a regex pattern, compiling the regex once on first use.
#[cfg(feature = "regex")]
#[doc(hidden)]
#[macro_export]
macro_rules! __validate_pattern {
    ($value:expr, $pattern:literal, $property:expr) => {{
        static REGEX: ::std::sync::LazyLock<$crate::private::regex::Regex> =
            ::std::sync::LazyLock::new(|| {
                $crate::private::regex::Regex::new($pattern)
                    .expect("pattern should be a valid regex")
            });
        $crate::private::validation::pattern($value, &REGEX, $pattern, $property)
    }};
}

#[cfg(not(feature = "regex"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __validate_pattern {
    ($($tt:tt)*) => {
        ::std::compile_error!("validating `pattern` requires the `regex` feature of `autapi`")
    };
}
